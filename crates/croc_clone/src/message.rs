use bincode::{Decode, Encode};

/// Message is the possible payload for messaging
#[derive(Debug, Encode, Decode)]
pub enum Message {
    PAKE { pake: Vec<u8>, curve: Vec<u8> },
    ExternalIP { external: String, bytes: Vec<u8> },
    Banner(Vec<u64>),
    Finished,
    Error(String),
    CloseRecipient,
    CloseSender,
    RecipientReady(RemoteFileRequest),
    FileInfo(SenderInfo),
}

/// SenderInfo lists the files to be transferred
#[derive(Debug, Encode, Decode)]
pub struct SenderInfo {
    to_transfer: Vec<FileInfo>,
    empty_dirs_to_transfer: Vec<FileInfo>,
    total_no_folders: i32,
    machine_id: String,
    ask: bool,
    sending_text: bool,
    no_compress: bool,
    hashed: bool,
}

#[derive(Encode, Decode, Debug)]
/// FileInfo registers the information about the file
pub struct FileInfo {
    name: String,
    folder_remote: String,
    folder_source: String,
    hash: Vec<u8>,
    size: i64,
    mod_time: std::time::SystemTime,
    is_compressed: bool,
    is_encrypted: bool,
    symlink: String,
    mode: u32,
    temp_file: bool,
}

#[derive(Encode, Decode, Debug)]
pub struct RemoteFileRequest {
    current_file_chunk_ranges: Vec<i64>,
    files_to_transfer_current_num: i32,
    machine_id: String,
}
