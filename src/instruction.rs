use borsh::BorshDeserialize;
use solana_program::program_error::ProgramError;

pub enum ProgramInstruction {
    // set counter to 0 in the base layer (main Solana Layer 1 blockchain)
    InitCounter,
    // Increments the counter (can be done on base or ephemeral rollup).
    IncreaseCounter { increase_by: u64 },
    // Moves the counter state from base layer to Ephemeral Rollup.
    Delegate,
    // Pushes state from ER → base layer, and undelegates it (maybe back to the user?)
    // done using the Rollup. Push the final state to L1 and return ownership/control to the base layer (or user).
    CommitAndUndelegate,
    // Pushes state from ER → base layer
    // push my latest counter value from the Rollup back to L1, but keep it delegated as still using the ER
    Commit,
    // Undelegates using PDA seeds. Vec<Vec<u8>> allows multiple seeds of varying lengths
    Undelegate { pda_seeds: Vec<Vec<u8>> },
}

#[derive(BorshDeserialize)]
struct IncreaseCounterPayload {
    increase_by: u64,
}

impl ProgramInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < 8 {
            return Err(ProgramError::InvalidInstructionData);
        };

        //let input: &[u8] = &[
        //    1, 0, 0, 0, 0, 0, 0, 0,   // <-- discriminator for IncreaseCounter || tell what instruction this is
        //    10, 0, 0, 0, 0, 0, 0, 0   // <-- increase_by = 10 (u64) || tell how to execute it (how much to inscrement)
        //];
        //
        //ix_discriminator = [1, 0, 0, 0, 0, 0, 0, 0]  // tells you it's `IncreaseCounter`
        //rest            = [10, 0, 0, 0, 0, 0, 0, 0] // payload for that instruction

        let (ix_discriminator, rest) = input.split_at(8);

        Ok(match ix_discriminator {
            [0, 0, 0, 0, 0, 0, 0, 0] => Self::InitCounter,
            [1, 0, 0, 0, 0, 0, 0, 0] => {
                // rest is the remaining bytes after the 8-byte instruction discriminator has been split off from the input data.
                // IncreaseCounterPayload::try_from_slice(rest) attempts to deserialize these remaining
                // bytes into an IncreaseCounterPayload struct according to the Borsh serialization format.

                //[10, 0, 0, 0, 0, 0, 0, 0]
                //
                //...into:
                //
                //IncreaseCounterPayload { increase_by: 10 } || here is were deserialization happening
                let payload = IncreaseCounterPayload::try_from_slice(rest)?;
                Self::IncreaseCounter {
                    increase_by: payload.increase_by,
                }
            }
            [2, 0, 0, 0, 0, 0, 0, 0] => Self::Delegate,
            [3, 0, 0, 0, 0, 0, 0, 0] => Self::CommitAndUndelegate,
            [4, 0, 0, 0, 0, 0, 0, 0] => Self::Commit,
            [196, 28, 41, 206, 48, 37, 51, 167] => {
                let pda_seeds: Vec<Vec<u8>> = Vec::<Vec<u8>>::try_from_slice(rest)?;
                Self::Undelegate { pda_seeds }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}
