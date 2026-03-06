use std::net::TcpListener;

pub fn port_available(host: &str, port: u16) -> bool {
    TcpListener::bind((host, port)).is_ok()
}
