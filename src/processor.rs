use crate::{instruction::ProgramInstruction, state::Counter};
use borsh::{BorshDeserialize, BorshSerialize};
use ephemeral_rollups_sdk::{
    cpi::{delegate_account, undelegate_account, DelegateAccounts, DelegateConfig},
    ephem::{commit_accounts, commit_and_undelegate_accounts},
};
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
    let instruction = ProgramInstruction::unpack(instruction_data)?;
    match instruction {
        // 0: InitializeCounter
        ProgramInstruction::InitCounter => process_init_counter(program_id, accounts),

        //1: IncreaseCounter
        ProgramInstruction::IncreaseCounter { increase_by } => {
            process_increase_counter(program_id, accounts, increase_by)
        }

        //2: Delegate
        ProgramInstruction::Delegate => process_delegation(program_id, accounts),

        //3: CommitAndUndelegate
        ProgramInstruction::CommitAndUndelegate => {
            process_commit_and_undelegate(program_id, accounts)
        }

        //4: Commit
        ProgramInstruction::Commit => process_commit(program_id, accounts),

        //5: Undelegate
        ProgramInstruction::Undelegate { pda_seeds } => undelegate(program_id, accounts, pda_seeds),
    }
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

pub fn process_increase_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    increase_by: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let initializer_acc = next_account_info(accounts_iter)?;
    let counter_acc = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;

    let (counter_pda, _bump) =
        Pubkey::find_program_address(&[b"counter_acc", initializer_acc.key.as_ref()], program_id);

    // checking if pda matchs with the counter_acc.key
    if counter_pda != *counter_acc.key {
        return Err(ProgramError::InvalidArgument);
    }

    let mut counter_data = Counter::try_from_slice(&counter_acc.data.borrow())?;
    counter_data.count += increase_by;
    counter_data.serialize(&mut &mut counter_acc.data.borrow_mut()[..])?;

    Ok(())
}

pub fn process_delegation(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let initializer = next_account_info(account_info_iter)?; // user who owns the counter
    let pda_to_delegate = next_account_info(account_info_iter)?; // the actual counter PDA you're delegating
    let owner_program = next_account_info(account_info_iter)?; // program (the one managing the counter)
    let delegation_buffer = next_account_info(account_info_iter)?; // temporary buffer for update queue
    let delegation_record = next_account_info(account_info_iter)?; // record that stores delegation state
    let delegation_metadata = next_account_info(account_info_iter)?; // extra metadata like lifetime, status
    let delegation_program = next_account_info(account_info_iter)?; // magicblock's delegation program
    let system_program = next_account_info(account_info_iter)?; // creating accounts if needed

    let seed_1 = b"counter_acc";
    let seed_2 = initializer.key.as_ref();
    let pda_seeds: &[&[u8]] = &[seed_1, seed_2];

    let delegate_accounts = DelegateAccounts {
        payer: initializer,
        pda: pda_to_delegate,
        owner_program,
        buffer: delegation_buffer,
        delegation_record,
        delegation_metadata,
        delegation_program,
        system_program,
    };

    let delegation_config = DelegateConfig {
        commit_frequency_ms: 30_000,
        validator: None,
    };

    delegate_account(delegate_accounts, pda_seeds, delegation_config)?;

    Ok(())
}

pub fn undelegate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    pda_seeds: Vec<Vec<u8>>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let delegate_pda = next_account_info(accounts_iter)?;
    let delegation_buffer = next_account_info(accounts_iter)?;
    let initializer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    undelegate_account(
        delegate_pda,
        program_id,
        delegation_buffer,
        initializer,
        system_program,
        pda_seeds,
    )?;

    Ok(())
}

pub fn process_commit_and_undelegate(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let initializer = next_account_info(account_info_iter)?;
    let counter_account = next_account_info(account_info_iter)?;
    let magic_program = next_account_info(account_info_iter)?;
    let magic_context = next_account_info(account_info_iter)?;

    // Signer should be the same as the initializer
    if !initializer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Commit and undelegate counter_account on ER
    commit_and_undelegate_accounts(
        initializer,
        vec![counter_account],
        magic_context,
        magic_program,
    )?;

    Ok(())
}

pub fn process_commit(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let initializer = next_account_info(account_info_iter)?;
    let counter_account = next_account_info(account_info_iter)?;
    let magic_program = next_account_info(account_info_iter)?;
    let magic_context = next_account_info(account_info_iter)?;

    // Signer should be the same as the initializer
    if !initializer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Commit counter_account on ER
    commit_accounts(
        initializer,
        vec![counter_account],
        magic_context,
        magic_program,
    )?;

    Ok(())
}
