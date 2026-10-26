//! Port of Vape-fabric's recovered n6.e and kB timing policies.
//! Source: pvputils/Vape-fabric, VapeAutoClickerTimingTodoAi / VapeSilentProfileTodoAi.
//! Java RNG arithmetic is retained so seeded source traces remain comparable.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Randomization {
    #[default]
    Citron,
    VapeNormal,
    VapeExtra,
    VapeExtraPlus,
}

impl Randomization {
    pub const ALL: [Self; 4] = [
        Self::Citron,
        Self::VapeNormal,
        Self::VapeExtra,
        Self::VapeExtraPlus,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Citron => "citron",
            Self::VapeNormal => "vape-normal",
            Self::VapeExtra => "vape-extra",
            Self::VapeExtraPlus => "vape-extra+",
        }
    }
}

const GAMMA: u64 = 0x9e3779b97f4a7c15;

fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

struct JavaRandom(u64);
impl JavaRandom {
    fn new(seed: u64) -> Self {
        Self((seed ^ 0x5deece66d) & ((1 << 48) - 1))
    }
    fn bits(&mut self, n: u32) -> u32 {
        self.0 = self.0.wrapping_mul(0x5deece66d).wrapping_add(11) & ((1 << 48) - 1);
        (self.0 >> (48 - n)) as u32
    }
    fn int(&mut self, bound: i32) -> i32 {
        let n = bound as u32;
        if n.is_power_of_two() {
            return ((n as u64 * self.bits(31) as u64) >> 31) as i32;
        }
        loop {
            let bits = self.bits(31) as i32;
            let value = bits % bound;
            if bits.wrapping_sub(value).wrapping_add(bound - 1) >= 0 {
                return value;
            }
        }
    }
}

struct SplitRandom(u64);
impl SplitRandom {
    fn seed(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(GAMMA);
        self.0
    }
    fn unit(&mut self) -> f64 {
        (mix64(self.seed()) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn int(&mut self, bound: i32) -> i32 {
        let next = |s: &mut Self| {
            let mut z = s.seed();
            z = (z ^ (z >> 33)).wrapping_mul(0x62a9d9ed799705f5);
            ((z ^ (z >> 28)).wrapping_mul(0xcb24d0a5c88c35b3) >> 32) as i32
        };
        let mut r = next(self);
        let mask = bound - 1;
        if bound & mask == 0 {
            return r & mask;
        }
        loop {
            let u = ((r as u32) >> 1) as i32;
            let value = u % bound;
            if u.wrapping_add(mask).wrapping_sub(value) >= 0 {
                return value;
            }
            r = next(self);
        }
    }
}

pub struct VapeTiming {
    random: JavaRandom,
    profile: Profile,
    delay: i64,
    burst: bool,
    fatigue: bool,
    i: i32,
    j: i32,
    e: i32,
    u: i32,
    n: i32,
    v: i32,
}

impl VapeTiming {
    pub fn new(seed: u64) -> Self {
        Self {
            random: JavaRandom::new(seed),
            profile: Profile::new(seed),
            delay: 0,
            burst: false,
            fatigue: true,
            i: 0,
            j: 0,
            e: 0,
            u: 0,
            n: 0,
            v: 0,
        }
    }

    fn interval(&mut self, minimum: i32, maximum: i32, mode: Randomization, now: i64) -> i64 {
        let r = &mut self.random;
        let sampled = if maximum <= minimum {
            minimum
        } else {
            r.int(maximum - minimum) + minimum + 1
        };
        if mode == Randomization::VapeNormal {
            self.delay = 1000 / sampled as i64;
            return self.delay;
        }
        let sampled = sampled.max(1);
        if mode == Randomization::VapeExtra {
            if !self.burst {
                self.delay = 1000 / sampled as i64;
                if r.int(4) == 1 {
                    self.burst = true;
                    self.j = 1 + r.int(5);
                } else if r.int(10) != 1 && r.int(10) == 1 {
                    self.burst = true;
                    self.j = 5 + r.int(10);
                }
            }
            if self.burst {
                self.i += 1;
                if self.i >= self.j {
                    self.i = 0;
                    self.burst = false;
                }
            }
            if r.int(48) % (if self.fatigue { 6 } else { 10 }) == 0 && !self.burst {
                self.delay += (r.int(45) + 40) as i64;
            }
            if self.fatigue {
                self.e += 1;
                if self.e >= self.u {
                    self.n = 75 + r.int(125);
                    self.fatigue = false;
                    self.e = 0;
                }
                return self.delay + if r.int(5) == 3 { 50 } else { 25 };
            }
            self.v += 1;
            if self.v >= self.n {
                self.fatigue = true;
                self.u = 7 + r.int(8);
                self.v = 0;
            }
            return self.delay;
        }
        self.profile.configure(minimum, maximum);
        self.profile.interval(now)
    }

    /// Citron releases before waiting, then presses and waits. Return Vape's release/hold
    /// split in that order, retaining qU.K's -5ms adjustment and low-interval floor.
    pub fn get_delays(
        &mut self,
        min: f32,
        max: f32,
        mode: Randomization,
        now: i64,
        split: i32,
    ) -> (f64, f64) {
        let minimum = min.min(max).clamp(1.0, 25.0) as i32;
        let maximum = min.max(max).clamp(1.0, 25.0) as i32;
        let mut interval = self.interval(minimum, maximum, mode, now) - 5;
        if minimum <= 20 && interval <= 50 {
            interval = 45;
        }
        let d = (100 - interval.min(99) + 45) as f64 / 100.0;
        // Signed remainder is intentional: the source uses Random.nextInt() % 10.
        let fraction = (30.0 + (split % 10) as f64 + 40.0 * d) / 100.0;
        (
            (interval as f64 * fraction) as i64 as f64,
            (interval as f64 * (1.0 - fraction)) as i64 as f64,
        )
    }
}

struct Profile {
    w: i32,
    k: i32,
    e: f64,
    l: f64,
    q: f64,
    r: f64,
    low: f64,
    high: f64,
    fatigue: f64,
    j: f64,
    z: u64,
    t: i64,
    o: i64,
    a: i64,
    d: i64,
    random: SplitRandom,
    initialized: bool,
}

impl Profile {
    fn new(seed: u64) -> Self {
        let seed = mix64(seed ^ GAMMA);
        Self {
            w: 0,
            k: 0,
            e: 0.0,
            l: 0.06,
            q: 0.0,
            r: 0.0,
            low: 0.0,
            high: 0.0,
            fatigue: 0.0,
            j: 0.0,
            z: mix64(seed.wrapping_add(0xd1b54a32d192ed03)),
            t: 0,
            o: 0,
            a: 0,
            d: 0,
            random: SplitRandom(seed),
            initialized: false,
        }
    }
    fn configure(&mut self, minimum: i32, maximum: i32) {
        self.low = (minimum.max(0) as f64 * 1.5).max(0.5);
        self.high = (maximum.max(minimum.max(0)) as f64 * 1.5)
            .max(0.5)
            .max(self.low + 0.1);
        if !self.initialized {
            self.e = (self.low + self.high) * 0.5;
            self.j = (self.e + self.y() * 0.25).clamp(self.low, self.high);
        }
    }
    fn p(&mut self, now: i64, min: i64, max: i64) -> i64 {
        let value = ((now as u64 ^ self.z) & 0xffff) as i64;
        let range = max - min + 1;
        min + (value + self.random.int(range as i32) as i64) % range
    }
    fn schedule_target(&mut self, now: i64) {
        self.t = now + self.p(now, 1200, 3200);
    }
    fn schedule_drift(&mut self, now: i64) {
        self.d = now + self.p(now, 30000, 90000);
    }
    fn y(&mut self) -> f64 {
        let value: f64 = (0..6).map(|_| self.random.unit()).sum();
        (value - 3.0) * 1.22474487139
    }
    fn noise(&mut self) -> f64 {
        let sample = self.y();
        let pick = self.random.unit();
        if pick < 0.12 {
            return (self.random.unit() - 0.5) * 3.0;
        }
        if pick < 0.17 {
            return sample * (1.4 + self.random.unit() * 0.6);
        }
        sample
    }
    fn interval(&mut self, now: i64) -> i64 {
        if !self.initialized {
            self.initialized = true;
            self.schedule_target(now);
            self.schedule_drift(now);
            self.r = (self.random.unit() * 2.0 - 1.0) * 0.75;
            self.e = ((self.low + self.high) * 0.5 + self.r).clamp(self.low, self.high);
            self.o = now;
        }
        let elapsed = if self.o == 0 { 0 } else { now - self.o };
        if elapsed >= 1200 {
            let duration = elapsed.min(5000) as f64 / 10.0;
            self.fatigue = (self.fatigue - 0.004 * duration).max(0.0);
            self.w = if self.random.unit() < 0.7 {
                2 + self.random.int(4)
            } else {
                0
            };
            self.l = 0.03 + self.random.unit() * 0.06;
            self.q = self.random.unit() * 0.04;
        }
        self.k += 1;
        if self.k > 80 + self.random.int(120) {
            self.l = 0.03 + self.random.unit() * 0.06;
            self.q = self.random.unit() * 0.04;
            self.k = 0;
        }
        if now >= self.t {
            let sample = self.y();
            self.j += 0.25 * (self.e - self.j) + 0.45 * sample;
            if self.random.unit() < 0.03 {
                self.j += (self.random.unit() - 0.5) * 1.2;
            }
            self.j = self.j.clamp(self.low, self.high);
            self.schedule_target(now);
        }
        if now >= self.d {
            self.r = (self.random.unit() * 2.0 - 1.0) * 0.75;
            self.e = ((self.low + self.high) * 0.5 + self.r).clamp(self.low, self.high);
            self.schedule_drift(now);
        }
        let mut rate = self.j;
        let burst = self.w > 0;
        if burst {
            rate *= 1.0 + 0.05 * (0.4 + 0.8 * self.random.unit());
            self.w -= 1;
        }
        rate *= 1.0 - self.fatigue.min(0.4);
        rate = rate.clamp(self.low, self.high);
        let baseline = 1000.0 / rate;
        let multiplier = (0.24 * self.noise()).exp();
        let jitter = 35 + (self.random.int(11) - 5);
        let mut delay = baseline * multiplier + self.random.int(jitter.max(1) + 1) as f64;
        if burst {
            delay += self.random.int(15) as f64;
        }
        if self.a > 0 && self.random.unit() < self.l {
            delay = (self.a + self.random.int(7) as i64 - 3) as f64;
        }
        if self.random.unit() < 0.07 {
            delay *= 0.7 + self.random.unit() * 0.2;
        }
        let pause_chance = (0.04 + 0.12 * self.fatigue + self.q).min(0.18);
        if self.random.unit() < pause_chance {
            delay += (50 + self.random.int(101)) as f64;
        }
        self.fatigue = (self.fatigue + 0.015).min(0.4);
        self.o = now;
        self.a = delay.clamp(1.0, 2.147483647e9) as i64;
        self.a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovered_java_traces_match() {
        for row in include_str!("vapeReferenceTodoAi.csv")
            .lines()
            .filter(|s| !s.starts_with('#'))
        {
            let fields: Vec<_> = row.split(',').collect();
            let mode = [
                Randomization::VapeNormal,
                Randomization::VapeExtra,
                Randomization::VapeExtraPlus,
            ][fields[0].parse::<usize>().unwrap()];
            let minimum = fields[1].parse().unwrap();
            let maximum = fields[2].parse().unwrap();
            let mut timing = VapeTiming::new(fields[3].parse().unwrap());
            let mut now = 1_000_000;
            let mut hash = 0xcbf29ce484222325u64;
            for i in 0..600 {
                if i > 0 {
                    now += if i % 100 == 0 {
                        35000
                    } else if i % 37 == 0 {
                        1500
                    } else {
                        123 + i % 13
                    };
                }
                let interval = timing.interval(minimum, maximum, mode, now);
                for byte in interval.to_le_bytes() {
                    hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
                }
            }
            assert_eq!(
                hash,
                u64::from_str_radix(fields[4], 16).unwrap(),
                "Java trace {row}"
            );
        }
    }

    #[test]
    fn source_cycle_split_and_endpoint_casts() {
        for split in [-19, -9, 0, 9, 19] {
            for (lo, hi, expected_interval) in
                [(10.9, 10.9, 95_i64), (20.0, 20.0, 45), (25.0, 25.0, 35)]
            {
                let mut timing = VapeTiming::new(42);
                let (up, down) = timing.get_delays(lo, hi, Randomization::VapeNormal, 1000, split);
                let d = (100 - expected_interval.min(99) + 45) as f64 / 100.0;
                let fraction = (30.0 + (split % 10) as f64 + 40.0 * d) / 100.0;
                assert_eq!(up, (expected_interval as f64 * fraction) as i64 as f64);
                assert_eq!(
                    down,
                    (expected_interval as f64 * (1.0 - fraction)) as i64 as f64
                );
            }
        }
        let mut a = VapeTiming::new(42);
        let mut b = VapeTiming::new(42);
        for i in 0..1000 {
            assert_eq!(
                a.get_delays(13.9, 6.9, Randomization::VapeNormal, i, 0),
                b.get_delays(6.0, 13.0, Randomization::VapeNormal, i, 0)
            );
        }
    }

    #[test]
    fn modes_stay_finite_with_live_range_changes() {
        for mode in &Randomization::ALL[1..] {
            let mut timing = VapeTiming::new(17);
            for i in 0..3000 {
                let (lo, hi) = if i % 200 < 100 {
                    (1.0, 1.0)
                } else {
                    (21.0, 25.0)
                };
                let (up, down) = timing.get_delays(lo, hi, *mode, 1_000_000 + i * 100, i as i32);
                assert!(up.is_finite() && down.is_finite() && up > 0.0 && down > 0.0);
            }
        }
    }
}
