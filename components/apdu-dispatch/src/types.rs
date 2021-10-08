pub const SIZE: usize = 3072;
pub type Data = iso7816::Data<SIZE>;

interchange::interchange! {
    Contact: (Data, Data)
}

interchange::interchange! {
    Contactless: (Data, Data)
}
