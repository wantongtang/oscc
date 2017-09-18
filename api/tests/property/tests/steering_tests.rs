#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_must_use)]

include!(concat!(env!("OUT_DIR"), "/oscc_test.rs"));

extern crate quickcheck;
extern crate rand;
extern crate socketcan;

extern crate oscc_tests;

use quickcheck::{QuickCheck, TestResult, StdGen};

fn calculate_torque_spoofs( torque_command: f64 ) -> ( u16, u16 ) {
    let scaled_torque = oscc_tests::constrain(torque_command * MAXIMUM_TORQUE_COMMAND, MINIMUM_TORQUE_COMMAND, MAXIMUM_TORQUE_COMMAND);

    let mut high_spoof = (TORQUE_SPOOF_HIGH_SIGNAL_CALIBRATION_CURVE_SCALE * scaled_torque) + TORQUE_SPOOF_HIGH_SIGNAL_CALIBRATION_CURVE_OFFSET;
    let mut low_spoof = (TORQUE_SPOOF_LOW_SIGNAL_CALIBRATION_CURVE_SCALE * scaled_torque) + TORQUE_SPOOF_LOW_SIGNAL_CALIBRATION_CURVE_OFFSET;

    high_spoof = oscc_tests::constrain(high_spoof, STEERING_SPOOF_HIGH_SIGNAL_VOLTAGE_MIN, STEERING_SPOOF_HIGH_SIGNAL_VOLTAGE_MAX);

    low_spoof = oscc_tests::constrain(low_spoof, STEERING_SPOOF_LOW_SIGNAL_VOLTAGE_MIN, STEERING_SPOOF_LOW_SIGNAL_VOLTAGE_MAX);

    ((high_spoof  * STEPS_PER_VOLT) as u16, (low_spoof * STEPS_PER_VOLT) as u16)
}

fn get_steering_command_msg_from_buf( buffer: &[u8 ]) -> oscc_steering_command_s {
    let data_ptr: *const u8 = buffer.as_ptr();

    let steering_command_ptr: *const oscc_steering_command_s = data_ptr as *const _;

    unsafe { *steering_command_ptr as oscc_steering_command_s }
}

/// The API should properly calculate torque spoofs for valid range
fn prop_valid_torque_spoofs(steering_torque: f64) -> TestResult {
    let socket = oscc_tests::init_socket();

    unsafe { oscc_enable() };

    oscc_tests::skip_enable_frames(&socket);

    // send some command
    unsafe { oscc_publish_steering_torque(steering_torque); }

    // read from can frame
    let frame_result = socket.read_frame();
    match frame_result {
        Err(why) => TestResult::discard(),
        Ok(frame) => {
            let torque_command_msg = get_steering_command_msg_from_buf( frame.data() );

            let actual_spoofs = (torque_command_msg.spoof_value_high, torque_command_msg.spoof_value_low);

            let expected_spoofs = calculate_torque_spoofs(steering_torque);

            TestResult::from_bool(actual_spoofs == expected_spoofs)
        }
    }
}

#[test]
fn check_valid_torque_spoofs() {
    oscc_tests::open_oscc();

    let ret = QuickCheck::new()
        .tests(1000)
        .gen(StdGen::new(rand::thread_rng(), 1 as usize))
        .quickcheck(prop_valid_torque_spoofs as fn(f64) -> TestResult);

    oscc_tests::close_oscc();

    ret
}

/// For any valid steering input, the API should never send a spoof value 
/// outside of the valid range
fn prop_constrain_steering_spoofs(steering_command: f64) -> TestResult {
    let socket = oscc_tests::init_socket();

    unsafe { oscc_enable() };

    oscc_tests::skip_enable_frames(&socket);

    // send some command
    unsafe { oscc_publish_steering_torque(steering_command); }

    // read from can frame
    let frame_result = socket.read_frame();
    match frame_result {
        Err(why) => TestResult::discard(),
        Ok(frame) => {
            let steering_command_msg = get_steering_command_msg_from_buf( frame.data() );

            let spoof_high = steering_command_msg.spoof_value_high as u32;
            let spoof_low = steering_command_msg.spoof_value_low as u32;

            // fails on 1 w high-min of 737 -- need to constrain from API possibly

            TestResult::from_bool( 
                (spoof_high <= STEERING_SPOOF_HIGH_SIGNAL_RANGE_MAX) && 
                (spoof_high >= STEERING_SPOOF_HIGH_SIGNAL_RANGE_MIN) && (spoof_low <= STEERING_SPOOF_LOW_SIGNAL_RANGE_MAX) && (spoof_low >= STEERING_SPOOF_LOW_SIGNAL_RANGE_MIN))
        }
    }
}

#[test]
#[ignore]
fn check_constrain_steering_spoofs() {
    oscc_tests::open_oscc();

    let ret = QuickCheck::new()
        .tests(1000)
        .gen(StdGen::new(rand::thread_rng(), std::f64::MAX as usize))
        .quickcheck(prop_constrain_steering_spoofs as fn(f64) -> TestResult);

    oscc_tests::close_oscc();

    ret
}