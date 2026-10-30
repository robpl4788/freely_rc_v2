
#[repr(u8)]
enum OutputMode {
    PushPull = 0,
    OpenDrain = 1,
}

enum DriveCapability {
    Normal,
    Large,
    Huge, // HDRV = 0
}

pub struct OutputConfig {
    
}

pub struct Output {

}