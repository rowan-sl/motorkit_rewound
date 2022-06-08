use linux_embedded_hal::I2cdev;
use pwm_pca9685::{Pca9685, Address};

pub mod stepper;
pub mod dc;

pub mod error {
    #[derive(Debug, Clone, thiserror::Error, serde::Serialize, serde::Deserialize)]
    /// A list of all errors that can be thrown by the library.
    pub enum MotorError {
        /// An error occurred initializing the I2C bus.
        #[error("An error occured initializing the I2C bus")]
        I2cError,
        /// An error occurred configuring the PCA9685.
        #[error("An error occured configuring the PCA9685")]
        PwmError,
        /// An error occurred setting a channel.
        #[error("An error occured setting a channel")]
        ChannelError,
        /// The value for throttle is not in the bounds of [-1.0, 1.0].
        #[error("The value for the throttle is not in the bounds of [-1.0, 1.0]")]
        ThrottleError,
        /// An invalid motor was provided to a constructor, i.e. a stepper motor
        /// passed into the DcMotor constructor.
        #[error("An invalid motor was provided to a constructor, i.e. a stepper motor passed into the DcMotor constructor!")]
        InvalidMotorError,
    }

    /// cannot be serialize bc of inner values
    #[derive(Debug, thiserror::Error)]
    pub enum InitError {
        #[error("Failed to initialize the i2c bus: {0:?}")]
        InitI2cErr(#[from] linux_embedded_hal::i2cdev::linux::LinuxI2CError),
        #[error("Failed to initialize the driver device: {0:?}")]
        InitDriverErr(pwm_pca9685::Error<linux_embedded_hal::i2cdev::linux::LinuxI2CError>),
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// An enumeration of all potential motors that can be controlled via the
/// Motor HAT.
pub enum Motor {
    Motor1,
    Motor2,
    Motor3,
    Motor4,
    Stepper1,
    Stepper2,
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