use clap::Parser;
use log::warn;
use rs4a_vapix::{
    apis::event1::{GetEventInstancesRequest, MessageInstance, SimpleItemDeclaration},
    Client, ClientBuilder,
};
use url::Host;

/// Discover the event catalog exposed by a device.
#[derive(Parser)]
struct Args {
    #[command(flatten)]
    netloc: Netloc,
    /// Only print events whose topic path contains this substring.
    /// If repeated, print events whose topic contains any one of the filters.
    #[arg(short, long)]
    filter: Vec<String>,
}

#[derive(clap::Args)]
struct Netloc {
    /// Hostname or IP address of the device.
    #[arg(long, value_parser = Host::parse, env = "AXIS_DEVICE_IP")]
    host: Host,
    /// Override the default port for HTTP.
    #[arg(long, env = "AXIS_DEVICE_HTTP_PORT")]
    http_port: Option<u16>,
    /// Override the default port for HTTPS.
    #[arg(long, env = "AXIS_DEVICE_HTTPS_PORT")]
    https_port: Option<u16>,
    /// The username to use for authentication.
    #[arg(short, long, env = "AXIS_DEVICE_USER", default_value = "root")]
    user: String,
    /// The password to use for authentication.
    #[arg(short, long, env = "AXIS_DEVICE_PASS", default_value = "pass")]
    pass: String,
    /// Accept self-signed HTTPS certificates.
    #[arg(long, env = "AXIS_DEVICE_HTTPS_SELF_SIGNED", value_parser = clap::builder::BoolishValueParser::new())]
    https_self_signed: bool,
}

impl Netloc {
    async fn connect(&self) -> anyhow::Result<Client> {
        ClientBuilder::new(self.host.clone())
            .plain_port(self.http_port)
            .secure_port(self.https_port)
            .username_password(&self.user, &self.pass)
            .with_inner(|b| b.danger_accept_invalid_certs(self.https_self_signed))
            .build()
            .await
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let mut guard = rs4a_bin_utils::logger::init();
    let args = Args::parse();

    let client = args.netloc.connect().await?;

    let response = GetEventInstancesRequest::new().send(&client).await?;

    for msg in &response.message_instances {
        if !args.filter.is_empty() && !args.filter.iter().any(|f| msg.topic.join("/").contains(f))
        {
            continue;
        }
        print_message_instance(msg);
    }

    guard.disarm();
    Ok(())
}

fn print_message_instance(msg: &MessageInstance) {
    let MessageInstance {
        topic,
        is_property,
        source,
        key,
        data,
    } = msg;

    const SEPARATOR: &str = "/";
    for segment in topic {
        if segment.contains(SEPARATOR) {
            warn!("Topic segment {segment:?} contains separator {SEPARATOR:?}");
        }
    }
    let topic = topic.join(SEPARATOR);

    let property_marker = match is_property {
        true => " [property]",
        false => "",
    };
    println!("{topic}{property_marker}");

    for (kind, decls) in [("source", source), ("key", key), ("data", data)] {
        for decl in decls {
            print_simple_item_declaration(kind, decl);
        }
    }

    println!();
}

fn print_simple_item_declaration(kind: &str, decl: &SimpleItemDeclaration) {
    let SimpleItemDeclaration {
        name,
        value_type,
        values,
        is_property_state,
    } = decl;

    let state_marker = match is_property_state {
        true => " [propertyState]",
        false => "",
    };
    if values.is_empty() {
        println!("  {kind:<6} {name}: {value_type}{state_marker}");
    } else {
        let values = values.join(", ");
        println!("  {kind:<6} {name}: {value_type} ∈ {{{values}}}{state_marker}");
    }
}
