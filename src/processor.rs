use crate::{instruction::ProgramInstruction, state::Counter};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = ProgramInstruction::unpack(instruction_data);

    //match instruction {
    //    ProgramInstruction::InitCounter => process_init_counter(program_id, accounts),
    //}
    Ok(())
}
fn process_init_counter(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter(); // creates a mutable iterator over the account list so we can call next_account_info() multiple times to get each account in order
    let initializer_acc = next_account_info(accounts_iter)?; // who is initializing the acc (user or can be an admin)
    let counter_acc = next_account_info(accounts_iter)?; // counter account
    let system_program = next_account_info(accounts_iter)?; // system program

    // pda
    let (counter_pda, bump) =
        Pubkey::find_program_address(&[b"counter_acc", initializer_acc.key.as_ref()], program_id);

    // checking if pda matchs with the counter_acc.key
    if counter_pda != *counter_acc.key {
        return Err(ProgramError::InvalidArgument);
    }
    // check if pda exist or not
    let borrow_lamports = counter_acc.try_borrow_lamports().unwrap();
    if *borrow_lamports == &mut 0 {
        // calculate rent
        let rent = Rent::get()?;
        let rent_lamport = rent.minimum_balance(Counter::USZIE);

        // in solana programs, account data (including lamports) is accessed through RefCell-based borrow guards (like Ref, RefMut), which enforce Rust's borrow rules at runtime.
        // when you do try_borrow_lamports(), we get a mutable borrow of the account's lamports.
        // if you don’t release this borrow before calling something like invoke_signed(...) that might mutate or borrow from the same account again, you'll get a runtime panic
        drop(borrow_lamports);

        invoke_signed(
            // instruction
            &system_instruction::create_account(
                initializer_acc.key,
                counter_acc.key,
                rent_lamport,
                Counter::USZIE.try_into().unwrap(),
                program_id,
            ),
            // account_infos
            &[
                initializer_acc.clone(),
                counter_acc.clone(),
                system_program.clone(),
            ],
            // signers_seeds
            &[&[b"counter_acc", initializer_acc.key.as_ref(), &[bump]]],
        )?;

        //counter_account.data.borrow() gives you readonly access to the raw account data (as bytes).
        //Counter::try_from_slice(...) uses Borsh deserialization to convert those bytes into your Rust
        let mut counter_data = Counter::try_from_slice(&counter_acc.data.borrow())?;
        // setting it to 0
        counter_data.count = 0;
        // This is where you save the struct back into the account:
        // counter_account.data.borrow_mut() gives write access to the account's data bytes.
        // The [..] converts it into a &mut [u8] slice.
        // You then call .serialize(...) from Borsh to write your struct back into the account data in Borsh format.
        // the double &mut &mut – that’s just because serialize() needs &mut dyn Write and you're passing a mutable slice.
        counter_data.serialize(&mut &mut counter_acc.data.borrow_mut()[..])?;
    };

    Ok(())
}
