mod draw;
mod util;
mod walk;

use anyhow::{Context, Error as AnyhowError, Result as AnyhowResult};
use draw::Drawable;
use futures::ready;
use futures::{Stream, StreamExt};
use gif::{Encoder, Frame};
use hyper::{
    http::HeaderValue,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server, StatusCode,
};
use pin_project::pin_project;
use std::time::Instant;
use std::{
    convert::Infallible,
    error::Error,
    net::SocketAddr,
    ops::Deref,
    sync::{Arc, RwLock},
    task::Poll,
    time::Duration,
};
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use util::WritableArcRwLock;

type RxErr = Arc<dyn Error + Send + Sync>;
type GifRx = watch::Receiver<Result<Frame<'static>, RxErr>>;

#[pin_project]
struct FrameStreamToGifStream<F> {
    #[pin]
    frames: F,
    width: i32,
    height: i32,
    data: WritableArcRwLock<Vec<u8>>,
    encoder: Encoder<WritableArcRwLock<Vec<u8>>>,
}

impl<F> FrameStreamToGifStream<F> {
    const BUFFER_CAPACITY: usize = 1024;
    fn buffer_with_base_capacity() -> Vec<u8> {
        let mut v = Vec::new();
        v.reserve(Self::BUFFER_CAPACITY);
        v
    }
}

impl<F: Stream<Item = Result<Frame<'static>, RxErr>>> Stream for FrameStreamToGifStream<F> {
    type Item = Result<Vec<u8>, RxErr>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut this = self.project();
        let frame = match ready!(this.frames.as_mut().poll_next(cx)) {
            Some(t) => match t {
                Ok(o) => o,
                Err(e) => return Poll::Ready(Some(Err(e))),
            },
            None => return Poll::Ready(None),
        };
        match this.encoder.write_frame(&frame) {
            Ok(()) => {}
            Err(e) => {
                let ret: Box<dyn Error + Send + Sync> = e.into();
                return Poll::Ready(Some(Err(ret.into())));
            }
        }
        // if this deadlocks i'm going to be extremely embarrassed
        let locked_data: &RwLock<_> = (*this.data).deref();
        let data = &mut *locked_data.write().unwrap();
        let ret = std::mem::replace(data, Self::buffer_with_base_capacity());
        Poll::Ready(Some(Ok(ret)))
    }
}

async fn frame_stream_to_gif_stream(
    frames: impl Stream<Item = Result<Frame<'static>, RxErr>>,
    width: i32,
    height: i32,
) -> AnyhowResult<impl Stream<Item = Result<Vec<u8>, RxErr>>> {
    let data = WritableArcRwLock::new(Vec::new());
    let mut encoder = Encoder::new(data.clone(), width as _, height as _, &[])
        .context("could not create encoder")?;
    encoder
        .set_repeat(gif::Repeat::Finite(1))
        .context("could not set gif repeat")?;
    Ok(FrameStreamToGifStream {
        encoder,
        data,
        frames,
        width,
        height,
    })
}

async fn serve_gifs(
    req: Request<Body>,
    gif_rx: GifRx,
    width: i32,
    height: i32,
) -> AnyhowResult<Response<Body>> {
    dbg!(&req);
    // for now just serve one gif
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut()
        .insert("Content-Type", HeaderValue::from_static("image/gif"));
    let frame_stream = WatchStream::new(gif_rx).skip(1); // remove the faulty initial value
    let gif_stream = frame_stream_to_gif_stream(frame_stream, width, height)
        .await?
        .inspect(|y| {
            if let Err(e) = y {
                eprintln!("error: {e}")
            }
        });
    // if it's discordbot, limit the frames, so it thinks it's valid
    if req
        .headers()
        .get(hyper::header::USER_AGENT)
        .and_then(|agent| agent.to_str().ok())
        .map_or(false, |agent| agent.contains("Discordbot"))
    {
        *resp.body_mut() = Body::wrap_stream(gif_stream.take(10));
    } else {
        *resp.body_mut() = Body::wrap_stream(gif_stream);
    }
    Ok(resp)
}

const FONT: &str = "/nix/store/1b71gd014rj0ws5l5yz82iizg71i8ss8-nerdfonts-2.3.0/share/fonts/truetype/NerdFonts/JetBrains Mono Regular Nerd Font Complete.ttf";

type ToDraw = draw::TimeDisplay;

#[tokio::main]
async fn main() -> AnyhowResult<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    let invalid_first_frame: Box<dyn Error + Send + Sync> =
        "this is the first frame, this is not an error".into();
    let (gif_tx, gif_rx): (_, GifRx) = watch::channel(Err(invalid_first_frame.into()));

    std::thread::spawn(move || -> AnyhowResult<Infallible> {
        let freetype = freetype::library::Library::init().expect("freetype library should work");
        let face = freetype
            .new_face(&FONT, 0)
            .context("could not get freetype face")?;
        let drawer = draw::Drawer::<ToDraw>::new(face)?;
        let frames = draw::animate_gif(drawer);
        for (frametime, next_frame) in frames {
            let before_proc = std::time::Instant::now();
            // required for Clone
            let next_frame = next_frame.map_err(|err| {
                eprintln!("{:#}", err);
                let boxed_err: Box<dyn Error + Send + Sync> = err.into();
                let arc_err: Arc<_> = boxed_err.into();
                arc_err
            });
            gif_tx.send(next_frame)?;
            let proc_time = before_proc.elapsed() + frametime;
            // 30 FPS
            std::thread::sleep(
                Duration::from_millis(1000 / draw::FPS as u64).saturating_sub(proc_time),
            );
            //println!("time to compute frame: {}ms", proc_time.as_millis());
        }
        Err(AnyhowError::msg("FRAME GENERATOR HALTED"))
    });

    let make_svc = make_service_fn(move |_conn| {
        let gif_rx = gif_rx.clone();
        async move {
            Ok::<_, AnyhowError>(service_fn(move |req| {
                serve_gifs(req, gif_rx.clone(), ToDraw::WIDTH as _, ToDraw::HEIGHT as _)
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
