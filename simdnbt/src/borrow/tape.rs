pub struct Tape {
    pub elements: Vec<TapeElement>,
}

#[repr(C)]
pub union TapeElement {
    kind: (TapeTagKind, TapeTagValue),

    long: i64,
    double: f64,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub union TapeTagValue {
    byte: i8,
    short: i16,
    int: i32,
    long: (), // value is in next tape element
    float: f32,
    double: (),                    // value is in next tape element
    byte_array: u56,               // pointer to the original data
    string: u56,                   // pointer to the original data
    compound: (u24, UnalignedU32), // length estimate + tape index to the end of the compound
    int_array: u56,                // pointer to the original data
    long_array: u56,               // pointer to the original data

    // lists
    empty_list: (),
    byte_list: u56,                     // pointer to the original data
    short_list: u56,                    // pointer to the original data
    int_list: u56,                      // pointer to the original data
    long_list: u56,                     // pointer to the original data
    float_list: u56,                    // pointer to the original data
    double_list: u56,                   // pointer to the original data
    byte_array_list: u56, // pointer to a fat pointer we keep elsewhere that points to the original data
    string_list: u56,     // pointer to a fat pointer
    list_list: (u24, UnalignedU32), // length estimate + tape index to the end of the list
    compound_list: (u24, UnalignedU32), // length estimate + tape index to the end of the list
    int_array_list: u56,  // pointer to a fat pointer
    long_array_list: u56, // pointer to a fat pointer
}

#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct u56 {
    a: u8,
    b: u16,
    c: u32,
}
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct u48 {
    a: u16,
    b: u32,
}
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct u24 {
    a: u8,
    b: u16,
}
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct UnalignedU32(pub u32);

#[derive(Clone, Copy)]
#[repr(u8)]
enum TapeTagKind {
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    ByteArray,
    String,
    Compound,
    IntArray,
    LongArray,

    EmptyList,
    ByteList,
    ShortList,
    IntList,
    LongList,
    FloatList,
    DoubleList,
    ByteArrayList,
    StringList,
    ListList,
    CompoundList,
    IntArrayList,
    LongArrayList,
}
