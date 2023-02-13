use std::f64::consts::TAU;

use rand::{thread_rng, Rng};

use nalgebra::Vector2;

type Polar<T> = Vector2<T>;

fn polar_to_cartesian(p: Polar<f64>) -> Vector2<f64> {
    let x = p[0] * p[1].cos();
    let y = p[0] * p[1].sin();
    [x,y].into()
}

// it's a random 'drive' instead of a random walk
// because the velocity's r and theta vary instead of the position.
// get it? get it?
pub struct RandomDrive {
    pub curr: Vector2<f64>,
    pub vel: Polar<f64>,
    // cartesian.
    pub topleft: Vector2<f64>,
    pub botright: Vector2<f64>,
    pub maxspeed: f64
}

pub fn vec_conv<T: Copy, U: From<T>>(inp: Vector2<T>) -> Vector2<U> where Vector2<U>: From<[U; 2]> {
    [inp[0].into(), inp[1].into()].into()
}

impl RandomDrive {
    fn new(start: Vector2<u32>, topleft: Vector2<u32>, botright: Vector2<u32>, maxspeed: f64) -> Self {
        Self {
            curr: vec_conv(start),
            topleft: vec_conv(topleft),
            botright: vec_conv(botright),
            vel: [0.,0.].into(),
            maxspeed
        }
    }
}

impl Iterator for RandomDrive {
    type Item = Vector2<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let vel_xy = &mut *self.vel;
        let vel_r = &mut vel_xy.x;
        let vel_theta = &mut vel_xy.y;

        const D_R: f64 = 0.5;
        const D_THETA: f64 = 0.4;
        *vel_r += thread_rng().gen_range(-D_R..D_R);
        *vel_theta += thread_rng().gen_range(-D_THETA..D_THETA);
        if *vel_r > self.maxspeed {
            *vel_r = self.maxspeed;
        }
        if *vel_r < 0. {
            *vel_r = -*vel_r;
            *vel_theta = -*vel_theta;
        }
        *vel_theta = vel_theta.rem_euclid(TAU);
        todo!()
    }
}

