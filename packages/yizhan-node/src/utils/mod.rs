use network_interface::{NetworkInterface, NetworkInterfaceConfig};

#[test]
pub(crate) fn find_mac_addr() {
    let network_interfaces = NetworkInterface::show().unwrap();
    for interface in network_interfaces {
        println!("==> interface {:?}", interface.mac_addr);
    }
}
