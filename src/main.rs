mod error;


fn q1() -> i32 {
    let mut total = 0;
    for i in 100..=200 {
        total += i;
    }
    total
}

fn q2() -> i32 {
    let mut total = 0;
    for i in (1001..=2001).step_by(2) {
        total += i;
    }
    total
}

fn q3() -> i32 {
    let mut total = 0;
    for i in (0..=50).step_by(2) {
        total += i;
    }
    total
}

fn q4() -> f32 {
    let mut total: f32 = 0.0;
    for i in 2..=100 {
        total = total + 1.0 / (i as f32);
    }
    total
}

fn q5() -> f32 {
    let mut total: f32 = 0.0;
    for i in 2..50{
        total += ((i as f32)-1.0)/(i as f32);
    }
    total
}

fn q6() -> i32 {
    let mut total: i32 = 0;
    for _ in 1..=10{
        let num = read_i32();
        total += num;
    }
    total / 10
}

fn q7() -> i32 {
    let mut biggest: i32 = 0;
    for i in 1..=10{
        let num = read_i32();
        if i == 1 {
            biggest = num;
        }
        else if biggest < num {
            biggest = num;
        }
    }
    biggest
}

fn q8() -> f32 {
    let mut total: f32 = 24.0;
    for _ in 1625..=2026{
        total *= 1.03;
    }
    total
}

fn q9() -> i64 {
    let input: i32 = read_i32();
    let mut total: i64 = 1;
    for i in 1..=input {
        total *= i as i64;
    }
    total
}

fn main() {
    let x = 5;
    println!("{}", x);
    println!("{}", q1());
    println!("{}", q2());
    println!("{}", q3());
    println!("{}", q4());
    println!("{}", q5());
    println!("{}", q6());
    println!("{}", q7());
    println!("{}", q8());
    println!("{}", q9());
}

use std::io;

fn read_input() -> String {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input
}

fn read_i32() -> i32 {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().parse().expect("Not an integer")
}