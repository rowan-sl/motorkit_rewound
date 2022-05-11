use linux_embedded_hal::I2cdev;
use pwm_pca9685::{Pca9685, Address};

compile_error!("read the readme");

pub mod error {
    #[derive(Debug, thiserror::Error)]
    pub enum InitError {
        #[error("Failed to initialize the i2c bus: {0:?}")]
        InitI2cErr(#[from] linux_embedded_hal::i2cdev::linux::LinuxI2CError),
        #[error("Failed to initialize the driver device: {0:?}")]
        InitDriverErr(pwm_pca9685::Error<linux_embedded_hal::i2cdev::linux::LinuxI2CError>),
    }
}

/// Initializes the PWM to control the Motor HAT. This makes a few assumptions:
/// - Assumes only one Motor HAT as 0x96.
/// - Assumes only a pre-scale of 4 so the HAT is running at ~1600 Hz.
///
/// If no I2C bus is provided to the function, it will attempt to
/// connect to /dev/i2c-1 which will work in most cases.
pub fn init_pwm(i2c: Option<I2cdev>) -> Result<Pca9685<I2cdev>, error::InitError> {
    let i2c = if let Some(i2c) = i2c {
        i2c
    } else {
        I2cdev::new("/dev/i2c-1")?
    };

    // The default address for the motor hat is 0x96.
    let address = Address::from(0x96);

    let mut pwm = Pca9685::new(i2c, address).map_err(|err| error::InitError::InitDriverErr(err))?;
    pwm.enable().map_err(|err| error::InitError::InitDriverErr(err))?;
    pwm.set_prescale(4).map_err(|err| error::InitError::InitDriverErr(err))?;
    Ok(pwm)
}