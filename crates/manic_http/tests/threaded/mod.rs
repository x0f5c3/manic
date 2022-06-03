mod local_threaded;
mod remote_threaded;

pub(crate) fn start_threaded(port: u16, srv: Option<&'static str>, file: Option<&'static str>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        rt.block_on(crate::start_server(port, srv, file));
    });
}
