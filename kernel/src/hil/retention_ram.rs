use crate::ErrorCode;

// pub enum RetentionError {
//     OutOfRange,
//     NotInintialized,
//     Failed,
// }
pub trait OwnerRetentionRam {
    type Data: Copy;
    type ID: Copy;

    fn read(&self, id: Self::ID) -> Result<Self::Data, ErrorCode>;

    fn write(&self, id: Self::ID, data: Self::Data) -> Result<(), ErrorCode>;
}

pub trait CreatorRetentionRam {
    type Data: Copy;
    type ID: Copy;

    fn read(&self, id: Self::ID) -> Result<Self::Data, ErrorCode>;
}
