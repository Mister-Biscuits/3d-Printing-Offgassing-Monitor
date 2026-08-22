use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Polyline, PrimitiveStyle},
};

// one history slot per horizontal pixel column
pub const GRAPH_WIDTH: usize = 240;
const HEIGHT: i32 = 320;

pub struct ShortGraph {
    co2_history: [u16; GRAPH_WIDTH],
    voc_history: [u16; GRAPH_WIDTH],
    write_index: usize,
    filled: bool,
}

impl ShortGraph {
    pub fn new() -> Self {
        Self {
            co2_history: [0; GRAPH_WIDTH],
            voc_history: [0; GRAPH_WIDTH],
            write_index: 0,
            filled: false,
        }
    }

    // records a new sample, wrapping around once the buffer fills
    pub fn push(&mut self, co2: u16, voc: u16) {
        self.co2_history[self.write_index] = co2;
        self.voc_history[self.write_index] = voc;
        self.write_index = (self.write_index + 1) % GRAPH_WIDTH;
        if self.write_index == 0 {
            self.filled = true;
        }
    }

    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        display.clear(Rgb565::BLACK)?;

        let count = if self.filled { GRAPH_WIDTH } else { self.write_index };
        if count < 2 {
            return Ok(());
        }

        // autoscale each line against its own max so CO2 (ppm) and VOC (ppb) both fill the screen height
        let max_co2 = self.co2_history[..count].iter().copied().max().unwrap_or(1).max(1);
        let max_voc = self.voc_history[..count].iter().copied().max().unwrap_or(1).max(1);

        let mut co2_points = [Point::zero(); GRAPH_WIDTH];
        let mut voc_points = [Point::zero(); GRAPH_WIDTH];

        for i in 0..count {
            // walk the ring buffer oldest-to-newest so the trace scrolls left to right
            let idx = if self.filled { (self.write_index + i) % GRAPH_WIDTH } else { i };
            let x = i as i32;
            co2_points[i] = Point::new(x, HEIGHT - 1 - (self.co2_history[idx] as i32 * (HEIGHT - 1) / max_co2 as i32));
            voc_points[i] = Point::new(x, HEIGHT - 1 - (self.voc_history[idx] as i32 * (HEIGHT - 1) / max_voc as i32));
        }

        Polyline::new(&co2_points[..count])
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::BLUE, 1))
            .draw(display)?;

        Polyline::new(&voc_points[..count])
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::RED, 1))
            .draw(display)?;

        Ok(())
    }
}