use crate::db::Device;

pub fn envs(device: &Device) -> Vec<(String, String)> {
    rs4a_dut::Device::from(device.clone()).to_env()
}
