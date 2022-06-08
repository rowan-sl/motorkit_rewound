use crate::{Motor, error::MotorError};
use hal::I2cdev;
use linux_embedded_hal as hal;
use pwm_pca9685::{Channel, Pca9685};
use std::f32::consts::PI;

impl Motor {
    fn to_step_channels(self) -> StepChannels {
        match self {
            Motor::Stepper1 => {
                StepChannels {
                    ref_channel1: Channel::C8,
                    ref_channel2: Channel::C13,
                    ain1: Channel::C10,
                    ain2: Channel::C9,
                    bin1: Channel::C11,
                    bin2: Channel::C12,
                }
            }
            Motor::Stepper2 => {
                StepChannels {
                    ref_channel1: Channel::C2,
                    ref_channel2: Channel::C7,
                    ain1: Channel::C4,
                    ain2: Channel::C3,
                    bin1: Channel::C5,
                    bin2: Channel::C6,
                }
            }
            _ => panic!("not a stepper motor")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// A structure encapsulating the channels used to control a stepper motor.
struct StepChannels {
    ref_channel1: Channel,
    ref_channel2: Channel,
    ain1: Channel,
    ain2: Channel,
    bin1: Channel,
    bin2: Channel,
}

#[derive(PartialEq, Debug)]
pub enum StepDirection {
    Forward,
    Backward,
}

#[derive(Debug, PartialEq)]
pub enum StepStyle {
    Single,
    Double,
    Interleave,
    Microstep,
}

/// A structure to initialize and control a stepper motor.
pub struct StepperMotor {
    microsteps: i32,
    channels: StepChannels,
    curve: Vec<i32>,
    current_step: i32,
}

impl StepperMotor {
    /// Initializes the stepper motor. If `microsteps` is not specified, it
    /// defaults to 16.
    pub fn try_new(
        pwm: &mut Pca9685<I2cdev>,
        step_motor: Motor,
        microsteps: Option<u32>,
    ) -> Result<Self, MotorError> {
        let channels = step_motor.to_step_channels();
        let microsteps = microsteps.unwrap_or(16) as i32;
        let curve: Vec<i32> = (0..microsteps + 1)
            .map(|i| {
                let value = ((65535.0
                    * (PI / (2.0 * microsteps as f32) * i as f32).sin())
                .round() as i32
                    + 1)
                    >> 4;
                value.min(4095)
            })
            .collect();

        // Set the channels that we'll be using on.
        pwm.set_channel_on(channels.ref_channel1, 0)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_on(channels.ref_channel1, 0)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_on(channels.ain1, 0)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_on(channels.ain2, 0)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_on(channels.bin1, 0)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_on(channels.bin2, 0)
            .map_err(|_| MotorError::ChannelError)?;

        // Set the reference channels to full blast.
        pwm.set_channel_off(channels.ref_channel1, 4095)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_off(channels.ref_channel2, 4095)
            .map_err(|_| MotorError::ChannelError)?;

        let mut stepper = Self {
            microsteps,
            current_step: 0,
            channels,
            curve,
        };
        stepper.update_coils(pwm, [0; 4])?;
        Ok(stepper)
    }

    /// Commands the stepper motor to step one time in a given direction and
    /// with a given style.
    pub fn step_once(
        &mut self,
        pwm: &mut Pca9685<I2cdev>,
        step_dir: StepDirection,
        step_style: StepStyle,
    ) -> Result<(), MotorError> {
        // Set the reference channels to run at full blast.
        let duty_cycle = self.calc_step(step_dir, step_style)?;
        self.update_coils(pwm, duty_cycle)?;
        Ok(())
    }

    /// Stops energizing the PWMs for this motor.
    pub fn stop(
        &mut self,
        pwm: &mut Pca9685<I2cdev>,
    ) -> Result<(), MotorError> {
        pwm.set_channel_full_off(self.channels.ref_channel1)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_full_off(self.channels.ref_channel1)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_full_off(self.channels.ain1)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_full_off(self.channels.ain2)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_full_off(self.channels.bin1)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_full_off(self.channels.bin2)
            .map_err(|_| MotorError::ChannelError)?;
        Ok(())
    }

    fn calc_step(
        &mut self,
        step_dir: StepDirection,
        step_style: StepStyle,
    ) -> Result<[i32; 4], MotorError> {
        let step_size = self.calc_step_size(&step_dir, &step_style);
        match step_dir {
            StepDirection::Forward => self.current_step += step_size,
            StepDirection::Backward => self.current_step -= step_size,
        }
        //  ! this is the part where everything is fixed
        if self.current_step > (self.microsteps * 4) {
            self.current_step -= 40;
        } else if self.current_step < (-(self.microsteps * 4)) {
            self.current_step += 40;
        }
        let duty_cycles = self.calc_duty_cycle(step_style);
        Ok(duty_cycles)
    }

    fn update_coils(
        &mut self,
        pwm: &mut Pca9685<I2cdev>,
        duty_cycle: [i32; 4],
    ) -> Result<(), MotorError> {
        pwm.set_channel_off(self.channels.ain2, duty_cycle[0] as u16)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_off(self.channels.bin1, duty_cycle[1] as u16)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_off(self.channels.ain1, duty_cycle[2] as u16)
            .map_err(|_| MotorError::ChannelError)?;
        pwm.set_channel_off(self.channels.bin2, duty_cycle[3] as u16)
            .map_err(|_| MotorError::ChannelError)?;

        Ok(())
    }

    fn calc_duty_cycle(&mut self, step_style: StepStyle) -> [i32; 4] {
        let mut duty_cycles = [0; 4];
        let trailing_coil =
            ((self.current_step / self.microsteps) % 4) as usize;
        let leading_coil = ((trailing_coil + 1) % 4) as usize;
        let microstep = (self.current_step % self.microsteps) as usize;
        duty_cycles[leading_coil] = self.curve[microstep];
        duty_cycles[trailing_coil] =
            self.curve[self.microsteps as usize - microstep];
        if step_style != StepStyle::Microstep
            && duty_cycles[leading_coil] == duty_cycles[trailing_coil]
            && duty_cycles[leading_coil] > 0
        {
            duty_cycles[leading_coil] = 4095;
            duty_cycles[trailing_coil] = 4095;
        }
        duty_cycles
    }

    fn calc_step_size(
        &mut self,
        step_dir: &StepDirection,
        step_style: &StepStyle,
    ) -> i32 {
        if step_style == &StepStyle::Microstep {
            return 1;
        }

        let half_step = self.microsteps / 2;
        let additional_microsteps = self.current_step % half_step;
        if additional_microsteps != 0 {
            if step_dir == &StepDirection::Forward {
                self.current_step += half_step - additional_microsteps;
            } else {
                self.current_step -= additional_microsteps;
            }
            0
        } else if step_style == &StepStyle::Interleave {
            half_step
        } else {
            let curr_interleave = self.current_step / half_step;
            if (step_style == &StepStyle::Single && curr_interleave % 2 == 1)
                || (step_style == &StepStyle::Double
                    && curr_interleave % 2 == 0)
            {
                half_step
            } else {
                self.microsteps
            }
        }
    }
}
