pub const FRAME_LEN: usize = 1024 * 10;

pub type BitString = [u8; FRAME_LEN];

#[repr(C)]
pub struct ImageBuffer;

#[link(name = "framecoder")]
extern "C" {
    pub fn create_image_buffer(width: u16, height: u16, data: *mut u8) -> *mut ImageBuffer;

    pub fn release_resources(buffer: *mut ImageBuffer);

    pub fn delete_handle(buffer: *mut ImageBuffer);

    pub fn encode(input: *const BitString, write_buffer: *mut ImageBuffer);

    pub fn decode(output: *mut BitString, read_buffer: *const ImageBuffer);
}

#[test]
fn test_image_creation() {
    unsafe {
        let mut f_data = vec![1; 100 * 100];

        let buffer = create_image_buffer(100, 100, f_data.as_mut_ptr());

        delete_handle(buffer);
    }
}
