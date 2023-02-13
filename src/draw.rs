use std::time::{Duration, Instant};

use anyhow::{Context as AnyhowContext, Result};
use cairo::{self, Context, ImageSurface};
use freetype::Face;
use gif::Frame;
//use std::time::Instant;

/// in milliseconds
pub const FPS: usize = 25;

pub trait Drawable
where
    Self: Default + Sized,
{
    const WIDTH: usize;
    const HEIGHT: usize;
    fn draw_func(&mut self, info: &DrawInfo, ctx: &Context, frame_nr: usize) -> Result<()>;
}

#[derive(Default, Copy, Clone)]
pub struct TimeDisplay;
impl Drawable for TimeDisplay {
    const WIDTH: usize = 300;

    const HEIGHT: usize = 100;

    fn draw_func(&mut self, info: &DrawInfo, ctx: &Context, frame_nr: usize) -> Result<()> {
        ctx.set_source_rgb(1., 1., 1.);
        ctx.paint()?;

        ctx.set_source_rgb(0., 0., 0.);
        ctx.set_font_size(20.);
        ctx.move_to(0., 50.);
        ctx.show_text(&format!("this is frame #{frame_nr}"))
            .context("could not show text")?;
        Ok(())
    }
}

pub fn color_changing(ctx: &Context, frame: usize) -> Result<()> {
    let w = (frame % 128) as f64 / 128.;
    ctx.set_source_rgb(w, w, w);
    ctx.paint()?;
    Ok(())
}

// pub fn basic(ctx: &Context, frame: usize) -> Result<()> {
//     Ok(())
// }

pub struct DrawInfo {
    font: cairo::FontFace,
}

pub struct Drawer<D: Drawable> {
    info: DrawInfo,
    surface: ImageSurface,
    // rgba_buffer: Box<[u8]>,
    drawable: D,
}

impl<D: Drawable> Drawer<D> {
    pub fn new(face: Face) -> Result<Self> {
        let surface = ImageSurface::create(cairo::Format::Rgb24, D::WIDTH as _, D::HEIGHT as _)
            .context("couldn't create ImageSurface")?;
        Ok(Self {
            info: DrawInfo {
                font: cairo::FontFace::create_from_ft(&face).context("could not get cairo fontface")?,
            },
            surface,
            drawable: D::default(),
            // rgba_buffer: vec![0u8; D::WIDTH*D::HEIGHT*4].into_boxed_slice(),
        })
    }

    fn draw_frame(&mut self, frame_count: usize) -> Result<Frame<'static>> {
        {
            let ctx = Context::new(&self.surface).context("could not create cairo context")?;
            ctx.set_font_face(&self.info.font);
            self.drawable
                .draw_func(&self.info, &ctx, frame_count)
                .context("could not call drawing function")?;
        }
        self.surface.flush();
        let mut frame_data = self.surface.data().context("could not get surface data")?;
        // we don't have to change anything about the frame data, because
        // 1. this method actually uses ARGB data, which is great
        // 2. the &mut only clobbers alpha, which we don't use anyway, so that's also great
        // hence we can save time by just passing it directly
        Ok(Frame::from_rgba_speed(
            D::WIDTH as u16,
            D::HEIGHT as u16,
            &mut frame_data,
            /* speed */ 10,
        ))
    }
}

pub fn animate_gif<D: Drawable>(
    mut drawer: Drawer<D>,
) -> impl Iterator<Item = (Duration, Result<Frame<'static>>)> {
    (0..).map(move |i| {
        let before = Instant::now();
        let mut frame = drawer.draw_frame(i);
        if let Ok(frame) = &mut frame {
            // enforce fps
            // (units of 10 ms)
            frame.delay = (1000 / FPS as u16) / 10;
        }
        let frametime = before.elapsed();
        (frametime, frame)
    })
}
