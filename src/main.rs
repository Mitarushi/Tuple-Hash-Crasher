extern crate clap;
extern crate ahash;

use std::ops::Range;
use ahash::AHashMap;

use clap::{App, Arg};

// https://github.com/python/cpython/blob/80017752ba938852d53f9d83a404b4ecd9ff2baa/Objects/tupleobject.c#L392
const PYHASH_XXPRIME_1: u64 = 11400714785074694791;
const PYHASH_XXPRIME_2: u64 = 14029467366897019727;
const PYHASH_XXPRIME_5: u64 = 2870177450012600261;
const PYHASH_XXPRIME_1_INV: u64 = inverse_mod(PYHASH_XXPRIME_1);
const PYHASH_XXPRIME_2_INV: u64 = inverse_mod(PYHASH_XXPRIME_2);

// https://github.com/python/cpython/blob/caba55b3b735405b280273f7d99866a046c18281/Objects/tupleobject.c#L348
const PYHASH_PARAM_OLD_1: u64 = 0x345678;
const PYHASH_PARAM_OLD_2: u64 = 1000003;
const PYHASH_PARAM_OLD_3: u64 = 82520;
const PYHASH_PARAM_OLD_4: u64 = 97531;
const PYHASH_PARAM_OLD_2_INV: u64 = inverse_mod(PYHASH_PARAM_OLD_2);
const PYHASH_PARAM_OLD_MULT_INV: u64 = inverse_mod(PYHASH_PARAM_OLD_2.wrapping_add(PYHASH_PARAM_OLD_3).wrapping_add(2));

const fn inverse_mod(x: u64) -> u64 {
    let mut r = x;
    r = r.wrapping_mul(2u64.wrapping_sub(r.wrapping_mul(x)));
    r = r.wrapping_mul(2u64.wrapping_sub(r.wrapping_mul(x)));
    r = r.wrapping_mul(2u64.wrapping_sub(r.wrapping_mul(x)));
    r = r.wrapping_mul(2u64.wrapping_sub(r.wrapping_mul(x)));
    r = r.wrapping_mul(2u64.wrapping_sub(r.wrapping_mul(x)));
    r.wrapping_mul(2u64.wrapping_sub(r.wrapping_mul(x)))
}

fn PYHASH_XXROTATE(x: u64) -> u64 {
    (x << 31) | (x >> 33)
}

fn PYHASH_XXROTATE_REV(x: u64) -> u64 {
    (x >> 31) | (x << 33)
}

fn tuple_hash(item: &Vec<u64>) -> u64 {
    let length = item.len() as u64;

    let mut acc = PYHASH_XXPRIME_5;
    for i in item.iter() {
        acc = acc.wrapping_add(i.wrapping_mul(PYHASH_XXPRIME_2));
        acc = PYHASH_XXROTATE(acc);
        acc = acc.wrapping_mul(PYHASH_XXPRIME_1);
    }

    acc.wrapping_add(length ^ PYHASH_XXPRIME_5 ^ 3527539)
}

fn tuple_hash_old(item: &Vec<u64>) -> u64 {
    let mut length = item.len() as u64;

    let mut acc = PYHASH_PARAM_OLD_1;
    let mut mult = PYHASH_PARAM_OLD_2;
    for i in item.iter() {
        length = length.wrapping_sub(1);
        acc = (acc ^ i).wrapping_mul(mult);
        mult = mult.wrapping_add(PYHASH_PARAM_OLD_3).wrapping_add(length << 1);
    }

    acc.wrapping_add(PYHASH_PARAM_OLD_4)
}

struct RangeCounter<'a> {
    n: usize,
    range: &'a Range<u64>,
    counter: Vec<u64>,
}

impl<'a> RangeCounter<'a> {
    fn new(range: &'a Range<u64>, n: usize) -> RangeCounter {
        RangeCounter {
            n,
            range,
            counter: vec![range.start; n],
        }
    }

    fn next(&mut self) -> bool {
        if self.n == 0 {
            return true;
        }
        self.counter[0] += 1;
        for i in 0..self.n {
            if self.counter[i] == self.range.end {
                self.counter[i] = self.range.start;
                if i + 1 < self.counter.len() {
                    self.counter[i + 1] += 1;
                } else {
                    return true;
                }
            }
        }
        false
    }

    fn to_index(&self) -> u64 {
        let base = self.range.end - self.range.start;
        let mut index: u64 = 0;
        for i in self.counter.iter() {
            index = index.wrapping_mul(base).wrapping_add(i - self.range.start);
        }
        index
    }

    fn from_index(&self, index: u64) -> Vec<u64> {
        let mut index = index;
        let mut result = vec![0; self.n];
        let base = self.range.end - self.range.start;
        for i in (0..self.n).rev() {
            result[i] = index % base + self.range.start;
            index /= base;
        }
        result
    }
}

fn middle_lane_from_second(hash_last: u64, second_counter: &RangeCounter) -> u64 {
    let mut acc = hash_last;

    for i in second_counter.counter.iter().rev() {
        acc = acc.wrapping_mul(PYHASH_XXPRIME_1_INV);
        acc = PYHASH_XXROTATE_REV(acc);
        acc = acc.wrapping_sub(i.wrapping_mul(PYHASH_XXPRIME_2));
    }
    acc = acc.wrapping_mul(PYHASH_XXPRIME_1_INV);
    acc = PYHASH_XXROTATE_REV(acc);
    acc.wrapping_mul(PYHASH_XXPRIME_2_INV)
}

fn middle_lane_from_first(first_counter: &RangeCounter) -> u64 {
    let mut acc = PYHASH_XXPRIME_5;
    for i in first_counter.counter.iter() {
        acc = acc.wrapping_add(i.wrapping_mul(PYHASH_XXPRIME_2));
        acc = PYHASH_XXROTATE(acc);
        acc = acc.wrapping_mul(PYHASH_XXPRIME_1);
    }
    acc.wrapping_mul(PYHASH_XXPRIME_2_INV)
}

fn to_chunk(table: &Vec<(u64, u64)>, chunk_size: u64) -> AHashMap<u64, (usize, u8)> {
    let mut table_chunk_index: AHashMap<u64, (usize, u8)> = AHashMap::default();
    for i in 0..table.len() {
        let index = table[i].0 / chunk_size;
        if let Some(value) = table_chunk_index.get_mut(&index) {
            value.1 += 1;
        } else {
            table_chunk_index.insert(index, (i, 1));
        }
    }
    table_chunk_index
}

fn tuple_unhash_length_2<F>(hash_last: u64, item_range: &Range<u64>, number: usize, printer: F)
    where F: Fn(&Vec<u64>) {
    let mut cnt = 0;

    for i in item_range.clone() {
        let mut acc = hash_last;
        acc = acc.wrapping_mul(PYHASH_XXPRIME_1_INV);
        acc = PYHASH_XXROTATE_REV(acc);
        acc = acc.wrapping_sub(i.wrapping_mul(PYHASH_XXPRIME_2));

        acc = acc.wrapping_mul(PYHASH_XXPRIME_1_INV);
        acc = PYHASH_XXROTATE_REV(acc);
        let lane = acc.wrapping_sub(PYHASH_XXPRIME_5).wrapping_mul(PYHASH_XXPRIME_2_INV);

        if item_range.contains(&lane) {
            printer(&vec![lane, i]);

            cnt += 1;
            if cnt == number {
                return;
            }
        }
    }
}

fn tuple_unhash<F>(length: usize, hash: u64, item_range: &Range<u64>, table_size: usize,
                   number: usize, printer: F)
    where F: Fn(&Vec<u64>) {
    let hash_last = hash.wrapping_sub(length as u64 ^ PYHASH_XXPRIME_5 ^ 3527539);

    if length == 2 {
        tuple_unhash_length_2(hash_last, item_range, number, printer);
        return;
    }

    let second_half_length = (length - 1) / 2;
    let first_half_length = length - second_half_length - 1;
    let range_width = item_range.end - item_range.start;

    let mut second_counter = RangeCounter::new(item_range, second_half_length);

    let mut completed = false;
    let mut cnt = 0;
    while !completed {
        let mut table = Vec::with_capacity(table_size);

        for _ in 0..table_size {
            let lane = middle_lane_from_second(hash_last, &second_counter);

            table.push((lane, second_counter.to_index()));

            if second_counter.next() {
                completed = true;
                break;
            }
        }

        table.sort_unstable();

        let table_chunk_index = to_chunk(&table, range_width);

        let mut first_counter = RangeCounter::new(item_range, first_half_length);
        loop {
            let acc = middle_lane_from_first(&first_counter);

            let lower = acc.wrapping_add(item_range.start);
            let upper = acc.wrapping_add(item_range.end);

            let lower_chunk = lower / range_width;
            let upper_chunk = upper / range_width;

            let mut iterator = Vec::new();
            if let Some(&(a, b)) = table_chunk_index.get(&lower_chunk) {
                iterator.push(a..a + b as usize);
            }
            if let Some(&(a, b)) = table_chunk_index.get(&upper_chunk) {
                iterator.push(a..a + b as usize);
            }

            for iter in iterator {
                for index in iter {
                    let (lane, counter_index) = table[index];
                    if lower < upper {
                        if !(lower <= lane && lane < upper) {
                            continue;
                        }
                    } else {
                        if !(lower <= lane || lane < upper) {
                            continue;
                        }
                    }

                    let mut result = vec![0; length];

                    result[0..first_half_length].clone_from_slice(&first_counter.counter);
                    result[first_half_length] = lane.wrapping_sub(acc);
                    result[first_half_length + 1..].clone_from_slice(&second_counter.from_index(counter_index));

                    printer(&result);

                    cnt += 1;
                    if cnt == number {
                        return;
                    }
                }
            }

            if first_counter.next() {
                break;
            }
        }
    }
}

fn middle_lane_from_second_old(hash_last: u64, second_counter: &RangeCounter, length: u64) -> u64 {
    let mut acc = hash_last;
    let mut mult = PYHASH_PARAM_OLD_2.wrapping_add(length.wrapping_mul(PYHASH_PARAM_OLD_3));
    mult = mult.wrapping_add(length.wrapping_mul(length.wrapping_sub(1)));
    let mut length_2 = 0;

    for i in second_counter.counter.iter().rev() {
        mult = mult.wrapping_sub(PYHASH_PARAM_OLD_3).wrapping_sub(length_2 << 1);
        acc = acc.wrapping_mul(inverse_mod(mult)) ^ *i;
        length_2 += 1;
    }
    mult = mult.wrapping_sub(PYHASH_PARAM_OLD_3).wrapping_sub(length_2 << 1);
    acc.wrapping_mul(inverse_mod(mult))
}

fn middle_lane_from_first_old(first_counter: &RangeCounter, length: u64) -> u64 {
    let mut length = length;
    let mut acc = PYHASH_PARAM_OLD_1;
    let mut mult = PYHASH_PARAM_OLD_2;

    for i in first_counter.counter.iter() {
        length = length.wrapping_sub(1);
        acc = (acc ^ i).wrapping_mul(mult);
        mult = mult.wrapping_add(PYHASH_PARAM_OLD_3).wrapping_add(length << 1);
    }
    acc
}

fn prev_pow2_log(x: u64) -> u64 {
    63 - x.leading_zeros() as u64
}

fn tuple_unhash_length_2_old<F>(hash_last: u64, item_range: &Range<u64>, number: usize, printer: F)
    where F: Fn(&Vec<u64>) {
    let mut cnt = 0;

    for i in item_range.clone() {
        let acc = hash_last.wrapping_mul(PYHASH_PARAM_OLD_MULT_INV) ^ i;
        let lane = acc.wrapping_mul(PYHASH_PARAM_OLD_2_INV) ^ PYHASH_PARAM_OLD_1;

        if item_range.contains(&lane) {
            printer(&vec![lane, i]);

            cnt += 1;
            if cnt == number {
                return;
            }
        }
    }
}

fn tuple_unhash_old<F>(length: usize, hash: u64, item_range: &Range<u64>, table_size: usize,
                       number: usize, printer: F)
    where F: Fn(&Vec<u64>) {
    let hash_last = hash.wrapping_sub(PYHASH_PARAM_OLD_4);

    if length == 2 {
        tuple_unhash_length_2_old(hash_last, item_range, number, printer);
        return;
    }

    let second_half_length = (length - 1) / 2;
    let first_half_length = length - second_half_length - 1;
    let range_width_shift = prev_pow2_log(item_range.end - item_range.start);

    let mut second_counter = RangeCounter::new(item_range, second_half_length);

    let mut completed = false;
    let mut cnt = 0;
    while !completed {
        let mut table = Vec::with_capacity(table_size);

        for _ in 0..table_size {
            let lane = middle_lane_from_second_old(hash_last, &second_counter, length as u64);

            table.push((lane, second_counter.to_index()));

            if second_counter.next() {
                completed = true;
                break;
            }
        }

        table.sort_unstable();

        let table_chunk_index = to_chunk(&table, 1 << range_width_shift);

        let mut first_counter = RangeCounter::new(item_range, first_half_length);
        loop {
            let acc = middle_lane_from_first_old(&first_counter, length as u64);

            let lower = item_range.start;
            let upper = item_range.end;

            let lower_chunk = (lower ^ acc) >> range_width_shift;
            let upper_chunk = (upper ^ acc) >> range_width_shift;

            let mut iterator = Vec::new();
            if let Some(&(a, b)) = table_chunk_index.get(&lower_chunk) {
                iterator.push(a..a + b as usize);
            }
            if let Some(&(a, b)) = table_chunk_index.get(&upper_chunk) {
                iterator.push(a..a + b as usize);
            }

            for iter in iterator {
                for index in iter {
                    let (lane, counter_index) = table[index];
                    if lower < upper {
                        if !(lower <= (lane ^ acc) && (lane ^ acc) < upper) {
                            continue;
                        }
                    } else {
                        if !(lower <= (lane ^ acc) || (lane ^ acc) < upper) {
                            continue;
                        }
                    }

                    let mut result = vec![0; length];

                    result[0..first_half_length].clone_from_slice(&first_counter.counter);
                    result[first_half_length] = lane ^ acc;
                    result[first_half_length + 1..].clone_from_slice(&second_counter.from_index(counter_index));

                    printer(&result);

                    cnt += 1;
                    if cnt == number {
                        return;
                    }
                }
            }

            if first_counter.next() {
                break;
            }
        }
    }
}

fn raw_printer<const NEW: bool>(result: &Vec<u64>) {
    println!("{}", result.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(" "));
}

fn format_printer<const NEW: bool>(result: &Vec<u64>) {
    if NEW {
        println!("Tuple: {},\t Hash: {}", result.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(" "),
                 tuple_hash(result));
    } else {
        println!("Tuple: {},\t Hash: {}", result.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(" "),
                 tuple_hash_old(result));
    }
}


fn main() {
    let args = App::new("Tuple-Hash-Crasher")
        .version("0.1.0")
        .about("Generate tuple with same hash")
        .arg(Arg::new("length")
            .short('l')
            .long("length")
            .takes_value(true)
            .help("Length of tuple")
            .required(true))
        .arg(Arg::new("hash")
            .short('h')
            .long("hash")
            .takes_value(true)
            .help("Hash of tuple"))
        .arg(Arg::new("item_minimum")
            .short('m')
            .long("item_minimum")
            .takes_value(true)
            .help("Minimum value of item")
            .required(true))
        .arg(Arg::new("item_maximum")
            .short('M')
            .long("item_maximum")
            .takes_value(true)
            .help("Maximum value of item")
            .required(true))
        .arg(Arg::new("table_size")
            .short('t')
            .long("table_size")
            .takes_value(true)
            .help("Table size to run Algorithm")
            .required(true))
        .arg(Arg::new("number")
            .short('n')
            .long("number")
            .takes_value(true)
            .help("Number of tuple to generate"))
        .arg(Arg::new("old")
            .short('o')
            .long("old")
            .help("For previous version of Python implementation"))
        .arg(Arg::new("format")
            .short('f')
            .long("format")
            .help("Format tuple"));

    let matches = args.get_matches();

    let length = matches.value_of("length").unwrap().parse::<usize>().unwrap();
    let hash = if let Some(hash) = matches.value_of("hash") {
        hash.parse::<u64>().unwrap()
    } else {
        0
    };
    let item_minimum = matches.value_of("item_minimum").unwrap().parse::<u64>().unwrap();
    let item_maximum = matches.value_of("item_maximum").unwrap().parse::<u64>().unwrap();
    let table_size = matches.value_of("table_size").unwrap().parse::<usize>().unwrap();
    let number = if let Some(number) = matches.value_of("number") {
        number.parse::<usize>().unwrap()
    } else {
        usize::MAX
    };
    let new = !matches.is_present("old");
    let format = matches.is_present("format");

    let item_range = item_minimum..item_maximum;


    if new {
        const NEW: bool = true;
        let printer = if format {
            format_printer::<NEW>
        } else {
            raw_printer::<NEW>
        };
        tuple_unhash(length, hash, &item_range, table_size, number, printer);
    } else {
        const NEW: bool = false;
        let printer = if format {
            format_printer::<NEW>
        } else {
            raw_printer::<NEW>
        };
        tuple_unhash_old(length, hash, &item_range, table_size, number, printer);
    }
}
