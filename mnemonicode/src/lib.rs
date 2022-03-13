
use lazy_static::lazy_static;

pub fn words_required(len: i32) -> i32 {
    ((len + 1) * 3) / 4
}


enum EncTransState {
    NeedNothing,
    NeedPrefix,
    NeedWordSep,
    NeedGroupSep,
    NeedSuffix,
}

pub struct Config {
    line_prefix: String,
    line_suffix: String,
    word_seperator: String,
    group_seperator: String,
    words_per_group: u32,
    groups_per_line: u32,
    word_padding: char,
}

struct EncTrans {
    c: Config,
    state: EncTransState,
    word_cnt: u32,
    group_cnt: u32,
    wordidx: [i32;3],
    wordidxcnt: i32,
}

impl EncTrans {
    pub(crate) fn reset(&mut self) {
        self.state = EncTransState::NeedPrefix;
        self.word_cnt = 0;
        self.group_cnt = 0;
        self.wordidxcnt = 0;
    }
    pub(crate) fn str_state(&self) -> Option<&str> {
        let str_ret = match self.state {
            EncTransState::NeedPrefix => self.c.line_prefix.as_str(),
            EncTransState::NeedWordSep => self.c.word_seperator.as_str(),
            EncTransState::NeedGroupSep => self.c.group_seperator.as_str(),
            EncTransState::NeedSuffix => self.c.line_suffix.as_str(),
            EncTransState::NeedNothing => {}
        }
    }
}

lazy_static! {
    pub static ref DEFAULT_CONFIG: Config = Config {
        line_prefix: "".to_string(),
        line_suffix: "\n".to_string(),
        word_seperator: " ".to_string(),
        group_seperator: " - ".to_string(),
        words_per_group: 3,
        groups_per_line: 3,
        word_padding: ' '
    };
}