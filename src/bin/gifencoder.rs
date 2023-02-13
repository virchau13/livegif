use std::fs::File;
use ::gif::Encoder;
use gif::Frame;

fn main() {
    let f = File::create("f.gif").unwrap();
    let mut encoder = Encoder::new(f, 400, 400, &[]).unwrap();
    for i in 1..2u8 {
        println!("{i}");
        let mut img = [i; 400*400*4];
        let frame = Frame::from_rgba(400, 400, &mut img);
        encoder.write_frame(&frame).unwrap();
    }
}
