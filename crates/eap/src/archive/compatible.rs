use std::{
    ffi::OsStr,
    fs,
    io::{self, Write},
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::{Path, PathBuf},
};

use flate2::{Compression, GzBuilder};

use crate::Mtime;

const BLOCK_SIZE: usize = 512;
/// GNU tar's default blocking factor is 20 blocks;
/// archives are padded to a whole record of this size.
const RECORD_SIZE: usize = 20 * BLOCK_SIZE;
/// The largest file size this writer accepts: the 12-byte size header field holds 11 octal digits,
/// and the GNU base-256 extension for larger values is not implemented.
///
/// This limit coincides with [`Mtime::MAX`] because the two fields have the same width,
/// but it is enforced only here:
/// in equivalent mode GNU tar silently encodes larger sizes in base-256.
// TODO: Consider adding support for larger files
const MAX_SIZE: u64 = (1 << 33) - 1;

/// The files and directories excluded by `--exclude-vcs` in GNU tar 1.35.
// TODO: Consider exposing this concern on the library interface or even pushing it into callers
const VCS_NAMES: &[&str] = &[
    ".arch-ids",
    ".bzr",
    ".bzrignore",
    ".bzrtags",
    "CVS",
    ".cvsignore",
    "_darcs",
    ".git",
    ".gitattributes",
    ".gitignore",
    ".gitmodules",
    ".hg",
    ".hgignore",
    ".hgtags",
    "RCS",
    "SCCS",
    ".svn",
    "{arch}",
    "=RELEASE-ID",
    "=meta-update",
    "=update",
];

fn is_excluded(name: &OsStr) -> bool {
    // `--exclude '*~'`: any name ending in a tilde.
    if name.as_bytes().ends_with(b"~") {
        return true;
    }
    // `--exclude-vcs`.
    VCS_NAMES.iter().any(|vcs| OsStr::new(vcs) == name)
}

/// Writes `value` as left-zero-padded octal followed by a NUL into `field`.
fn put_octal(field: &mut [u8], value: u64) {
    let last = field.len() - 1;
    // The field holds `last` octal digits (the final byte is the NUL terminator); a larger value
    // would silently lose its high-order digits, producing a structurally valid but wrong header.
    // The GNU base-256 extension that lifts this limit is not implemented, so refuse instead.
    // The fields fed by external input are bounded: the mtime by the [`Mtime`] type and sizes
    // against [`MAX_SIZE`] by [`Tar::add`]; the remaining call sites are structurally
    // bounded, so tripping this assert is a bug.
    assert!(
        last >= 22 || value < 1 << (3 * last),
        "value {value} does not fit in {last} octal digits"
    );
    let mut remaining = value;
    #[expect(
        clippy::indexing_slicing,
        reason = "`last` is `field.len() - 1`, so the end of the range is in bounds"
    )]
    for slot in field[..last].iter_mut().rev() {
        *slot = b'0' + (remaining & 7) as u8;
        remaining >>= 3;
    }
    #[expect(
        clippy::indexing_slicing,
        reason = "`last` is `field.len() - 1`, so the index is in bounds"
    )]
    {
        field[last] = 0;
    }
}

/// Builds a single 512-byte GNU header block.
///
/// # Panics
///
/// Panics if either `name` and `linkname` don't fit in their 100-byte fields.
fn header(
    name: &[u8],
    mode: u32,
    size: u64,
    mtime: u64,
    typeflag: u8,
    linkname: &[u8],
) -> [u8; BLOCK_SIZE] {
    // Violations would not fail cleanly: a name of 101..=512 bytes would silently overwrite the
    // fields after the name field, producing a structurally plausible but wrong header.
    debug_assert!(
        name.len() <= 100,
        "caller must emit a LongLink record first"
    );
    debug_assert!(
        linkname.len() <= 100,
        "caller must emit a LongLink record first"
    );
    let mut h = [0u8; BLOCK_SIZE];
    #[expect(
        clippy::indexing_slicing,
        reason = "`name` fits in its 100-byte field per this function's contract, \
                    and the name field starts at offset 0 of the 512-byte block"
    )]
    h[..name.len()].copy_from_slice(name);
    put_octal(&mut h[100..108], u64::from(mode & 0o7777));
    put_octal(&mut h[108..116], 0); // uid (--owner 0)
    put_octal(&mut h[116..124], 0); // gid (--group 0)
    put_octal(&mut h[124..136], size);
    put_octal(&mut h[136..148], mtime);
    // The checksum is computed over the header with the checksum field itself
    // filled with spaces.
    h[148..156].fill(b' ');
    h[156] = typeflag;
    #[expect(
        clippy::indexing_slicing,
        reason = "`linkname` fits in its 100-byte field per this function's contract, \
                    so the range ends at most at offset 257 of the 512-byte block"
    )]
    h[157..157 + linkname.len()].copy_from_slice(linkname);
    // GNU magic and version: the eight bytes "ustar  \0".
    // uname, gname and the device fields are left zeroed (--numeric-owner).
    h[257..263].copy_from_slice(b"ustar ");
    h[263] = b' ';
    let checksum: u32 = h.iter().map(|&byte| u32::from(byte)).sum();
    // GNU writes the checksum as six octal digits, a NUL, then a space.
    put_octal(&mut h[148..155], u64::from(checksum));
    h[155] = b' ';
    h
}

/// GNU long-name marker entry type.
const GNUTYPE_LONGNAME: u8 = b'L';
/// GNU long-link-target marker entry type.
const GNUTYPE_LONGLINK: u8 = b'K';

struct Tar {
    out: Vec<u8>,
    mtime: u64,
}

impl Tar {
    fn push_block(&mut self, block: &[u8; BLOCK_SIZE]) {
        self.out.extend_from_slice(block);
    }

    /// Zero-pads the output up to the next multiple of `boundary`.
    fn pad_to(&mut self, boundary: usize) {
        self.out
            .resize(self.out.len().next_multiple_of(boundary), 0);
    }

    /// Appends `data` and zero-pads up to the next block boundary.
    fn push_data(&mut self, data: &[u8]) {
        self.out.extend_from_slice(data);
        self.pad_to(BLOCK_SIZE);
    }

    /// Emits a `././@LongLink` record carrying an over-long name or link target.
    fn push_long_name(&mut self, typeflag: u8, value: &[u8]) {
        // GNU stores mode 0644 and mtime 0 in the LongLink header, and the size
        // includes the trailing NUL that terminates the payload.
        let block = header(
            b"././@LongLink",
            0o644,
            (value.len() + 1) as u64,
            0,
            typeflag,
            b"",
        );
        self.push_block(&block);
        self.out.extend_from_slice(value);
        self.out.push(0);
        self.pad_to(BLOCK_SIZE);
    }

    fn append(
        &mut self,
        name: &[u8],
        mode: u32,
        size: u64,
        typeflag: u8,
        linkname: &[u8],
        data: Option<&[u8]>,
    ) {
        if linkname.len() > 100 {
            self.push_long_name(GNUTYPE_LONGLINK, linkname);
        }
        if name.len() > 100 {
            self.push_long_name(GNUTYPE_LONGNAME, name);
        }
        #[expect(
            clippy::indexing_slicing,
            reason = "the end of the range is clamped to `name.len()`"
        )]
        let name = &name[..name.len().min(100)];
        #[expect(
            clippy::indexing_slicing,
            reason = "the end of the range is clamped to `linkname.len()`"
        )]
        let linkname = &linkname[..linkname.len().min(100)];
        let block = header(name, mode, size, self.mtime, typeflag, linkname);
        self.push_block(&block);
        if let Some(data) = data {
            self.push_data(data);
        }
    }

    fn add(&mut self, root: &Path, rel: &Path) -> io::Result<()> {
        // GNU tar applies the exclude patterns to operands too, skipping matches silently (exit
        // status 0, no warning). Its unanchored patterns match a trailing run of path components,
        // and every pattern here is slash-free, so testing the last component is equivalent.
        if rel.file_name().is_some_and(is_excluded) {
            return Ok(());
        }
        let abs = root.join(rel);
        let metadata = abs.symlink_metadata()?;
        let file_type = metadata.file_type();
        let mode = metadata.permissions().mode();
        if file_type.is_symlink() {
            let target = fs::read_link(&abs)?;
            self.append(
                &tar_name(rel, false),
                mode,
                0,
                b'2',
                target.as_os_str().as_bytes(),
                None,
            );
        } else if file_type.is_dir() {
            self.append(&tar_name(rel, true), mode, 0, b'5', b"", None);
            // `--sort name`: children are archived in byte order, which is how
            // `OsString` compares on Unix.
            let mut names = fs::read_dir(&abs)?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<io::Result<Vec<_>>>()?;
            names.sort();
            for name in names {
                self.add(root, &rel.join(name))?;
            }
        } else {
            let data = fs::read(&abs)?;
            if data.len() as u64 > MAX_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "{abs:?} is {} bytes, but the size field holds at most {MAX_SIZE}",
                        data.len()
                    ),
                ));
            }
            self.append(
                &tar_name(rel, false),
                mode,
                data.len() as u64,
                b'0',
                b"",
                Some(&data),
            );
        }
        Ok(())
    }

    fn finish(mut self) -> Vec<u8> {
        // End-of-archive marker: two zero blocks, then pad to a whole record.
        self.out.resize(self.out.len() + 2 * BLOCK_SIZE, 0);
        self.pad_to(RECORD_SIZE);
        self.out
    }
}

fn tar_name(rel: &Path, is_dir: bool) -> Vec<u8> {
    let mut name = rel.as_os_str().as_bytes().to_vec();
    if is_dir {
        name.push(b'/');
    }
    name
}

/// Creates a GNU-format tar archive of `operands` (resolved relative to `root`).
///
/// `operands` are archived in the given order;
/// directories are recursed into with their entries sorted by name.
fn create_gnu_tar(root: &Path, operands: &[String], mtime: Mtime) -> io::Result<Vec<u8>> {
    let mut tar = Tar {
        out: Vec::new(),
        mtime: mtime.0,
    };
    for operand in operands {
        tar.add(root, Path::new(operand))?;
    }
    Ok(tar.finish())
}

/// Compresses `data` as gzip, deterministically and without any external process.
///
/// The pure-Rust `miniz_oxide` backend, a fixed mtime of 0, an empty file name and a pinned OS byte
/// make the output depend only on `data` and the compressor version.
/// It is therefore reproducible across platforms,
/// but it is *not* bit-identical to GNU `gzip` (and hence to the upstream `acap-build`).
fn gzip(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = GzBuilder::new()
        .operating_system(255)
        .write(Vec::new(), Compression::best());
    encoder.write_all(data)?;
    encoder.finish()
}

/// Builds the EAP entirely in process
///
/// The archive records the staged files' actual modes, so the output depends on the umask
/// in effect when the files were staged and on platform-specific symlink modes (0755 on
/// macOS, 0777 on Linux). Normalizing would remove this dependence, but mapping modes to
/// fixed values risks breaking apps that rely on special bits (e.g. setuid), so it needs
/// verification before it is enabled.
// TODO: Consider normalizing permission bits.
pub struct CompatibleArchiveBuilder {
    staging_dir: PathBuf,
    eap_file_name: String,
    mtime: Mtime,
    operands: Vec<String>,
}

impl CompatibleArchiveBuilder {
    pub fn new(staging_dir: &Path, eap_file_name: &str, mtime: Mtime) -> Self {
        Self {
            staging_dir: staging_dir.to_path_buf(),
            eap_file_name: eap_file_name.to_string(),
            mtime,
            operands: Vec::new(),
        }
    }

    pub fn files(&mut self, files: &[&str]) -> &mut Self {
        self.operands.extend(files.iter().map(|f| f.to_string()));
        self
    }

    /// Creates `eap_file_name` in `staging_dir` from the added files.
    pub fn finish(self) -> anyhow::Result<()> {
        let tar = create_gnu_tar(&self.staging_dir, &self.operands, self.mtime)?;
        let eap = gzip(&tar)?;
        fs::write(self.staging_dir.join(&self.eap_file_name), eap)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use tempfile::tempdir;

    use super::{super::EquivalentArchiveBuilder, *};

    const MTIME: Mtime = Mtime(1234567890);

    /// Returns the bytes a reference GNU `tar` produces for `operands`
    fn reference(root: &Path, operands: &[&str]) -> Vec<u8> {
        let mut tar = EquivalentArchiveBuilder::new_portable_without_compression(
            root,
            "__reference.tar",
            MTIME,
        )
        .unwrap();
        tar.files(operands);
        tar.run_with_logged_output().unwrap();
        fs::read(root.join("__reference.tar")).unwrap()
    }

    #[ignore = "requires a tier 2 developer environment"]
    #[test]
    fn matches_gnu_tar_byte_for_byte() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        // A regular file, an executable, and operands deliberately out of order
        // to confirm the command-line order is preserved.
        fs::write(root.join("zzz.conf"), b"conf\n").unwrap();
        let exe = root.join("myapp");
        fs::write(&exe, b"binary\n").unwrap();
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
        // A directory exercising sorted recursion, a symlink, and the excludes.
        let lib = root.join("lib");
        fs::create_dir(&lib).unwrap();
        fs::write(lib.join("zebra.txt"), b"z\n").unwrap();
        fs::write(lib.join("alpha.txt"), b"a\n").unwrap();
        symlink("alpha.txt", lib.join("link.txt")).unwrap();
        fs::write(lib.join("backup~"), b"drop\n").unwrap();
        fs::create_dir(lib.join(".git")).unwrap();
        fs::write(lib.join(".git").join("config"), b"drop\n").unwrap();
        // A name long enough to require a `././@LongLink` record.
        let long_name = "a".repeat(150);
        fs::write(lib.join(&long_name), b"long\n").unwrap();

        let operands = ["zzz.conf", "myapp", "lib"];
        let expected = reference(root, &operands);
        let actual = create_gnu_tar(root, &operands.map(str::to_string), MTIME).unwrap();

        assert_eq!(actual.len(), expected.len(), "archive lengths differ");
        assert_eq!(actual, expected, "archive bytes differ");
    }

    #[ignore = "requires a tier 2 developer environment"]
    #[test]
    fn excluded_operands_match_gnu_tar() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("myapp"), b"binary\n").unwrap();
        fs::write(root.join("notes~"), b"drop\n").unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join(".git").join("config"), b"drop\n").unwrap();

        // GNU tar archives only `myapp`, dropping the excluded file and directory operands.
        let operands = ["myapp", "notes~", ".git"];
        let expected = reference(root, &operands);
        let actual = create_gnu_tar(root, &operands.map(str::to_string), MTIME).unwrap();

        assert_eq!(actual, expected, "archive bytes differ");
    }
}
