pub const MAX_L_VALUE: u16 = 271;
pub const MAX_M_VALUE: u16 = 271;
pub const MAX_D_VALUE: u32 = 65_535;

pub const VN_HEADER_SIZE: u32 = 0x0C;

pub const EOS: u8 = 0x06;

// `VN_PAYLOAD_LIMIT`. As a concession to pragmatism we'll use a relatively small limit of
// 0x2000 bytes. This affords us cheap testing of all code paths. Large payloads can still be
// decoded using the overflow mechanism. We aren't expecting payloads much larger than 0x1000 bytes,
// so the real world performance cost is likely zero.
pub const VN_PAYLOAD_LIMIT: u32 = 0x2000;

// VN operation type.
//
// Key:
// L - literal length bit
// M - match length bit
// D - distance bit
// # - previous match distance

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Op {
    SmlL, // SmlL - 1110LLLL
    LrgL, // LrgL - 11100000 LLLLLLLL
    SmlM, // SmlM - 1111MMMM #
    LrgM, // LrgM - 11110000 MMMMMMMM #
    PreD, // PreD - LLMMM110 #
    SmlD, // SmlD - LLMMMDDD DDDDDDDD
    MedD, // MedD - 101LLMMM DDDDDDMM DDDDDDDD
    LrgD, // LrgD - LLMMM111 DDDDDDDD DDDDDDDD
    Eos,  // Eos  - End Of Stream.
    Udef, // Udef - Undefined.
    Nop,  // Nop  - No Operation.
}

#[rustfmt::skip]
pub const OP_TABLE: [Op; 0x100] = [
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::Eos,  Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::Nop,  Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::Nop,  Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::Udef, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::Udef, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::Udef, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::Udef, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::Udef, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef,
    Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD,
    Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD,
    Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD,
    Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD, Op::MedD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::SmlD, Op::PreD, Op::LrgD,
    Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef,
    Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef, Op::Udef,
    Op::LrgL, Op::SmlL, Op::SmlL, Op::SmlL, Op::SmlL, Op::SmlL, Op::SmlL, Op::SmlL,
    Op::SmlL, Op::SmlL, Op::SmlL, Op::SmlL, Op::SmlL, Op::SmlL, Op::SmlL, Op::SmlL,
    Op::LrgM, Op::SmlM, Op::SmlM, Op::SmlM, Op::SmlM, Op::SmlM, Op::SmlM, Op::SmlM,
    Op::SmlM, Op::SmlM, Op::SmlM, Op::SmlM, Op::SmlM, Op::SmlM, Op::SmlM, Op::SmlM,
];
