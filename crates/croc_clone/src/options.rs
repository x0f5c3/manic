// Options specifies user specific options
pub struct Options {
    issender: bool,
    sharedsecret: String,
    debug: bool,
    relayaddress: String,
    relayaddress6: String,
    relay_ports: Vec<u32>,
    relaypassword: String,
    stdout: bool,
    noprompt: bool,
    nomultiplexing: bool,
    disablelocal: bool,
    onlylocal: bool,
    ignorestdin: bool,
    ask: bool,
    sendingtext: bool,
    nocompress: bool,
    ip: String,
    overwrite: bool,
    curve: String,
    hashalgorithm: String,
    throttleupload: String,
    zipfolder: bool,
    testflag: bool,
}

pub struct Client {
    opts: Options,
    key: Vec<u8>,
    ext_ip: String,
    ext_ip_connected: String,
    step_1_channel_secured: bool,
    step2file_info_transferred: bool,
    step3recipient_request_file: bool,
    step4file_transferred: bool,
    step5close_channels: bool,
    successful_transfer: bool,

    // send / receive information of all files
    files_to_transfer: Vec<FileInfo>,
    empty_folders_to_transfer: Vec<FileInfo>,
    total_number_of_contents: i32,
    total_number_folders: i32,
    files_to_transfer_current_num: i32,
}

pub struct FileInfo {
    name: String,
    folder_remote: String,
    folder_source: String,
}
