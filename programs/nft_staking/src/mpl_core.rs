//! Minimal client for the Metaplex Core program.
//!
//! Core has no published Rust crate that lines up with this Anchor/Solana
//! toolchain, so instead of pulling in a conflicting dependency we build the
//! handful of instructions we need by hand. The borsh layouts and instruction
//! discriminators below mirror the Core program exactly.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
};

/// On-chain address of the Metaplex Core program.
pub const MPL_CORE_ID: Pubkey = Pubkey::from_str_const("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

// Instruction discriminators (first byte of the instruction data).
const IX_ADD_PLUGIN: u8 = 2;
const IX_ADD_COLLECTION_PLUGIN: u8 = 3;
const IX_REMOVE_PLUGIN: u8 = 4;
const IX_UPDATE_PLUGIN: u8 = 6;
const IX_UPDATE_COLLECTION_PLUGIN: u8 = 7;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FreezeDelegate {
    pub frozen: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct Attributes {
    pub attribute_list: Vec<Attribute>,
}

/// Core `Plugin` enum. Only the variants we use carry data; the others exist so
/// the borsh discriminant of each variant matches the Core program.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum Plugin {
    Royalties,
    FreezeDelegate(FreezeDelegate),
    BurnDelegate,
    TransferDelegate,
    UpdateDelegate,
    PermanentFreezeDelegate,
    Attributes(Attributes),
}

/// Core `PluginType` enum, used to identify a plugin for removal.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum PluginType {
    Royalties,
    FreezeDelegate,
}

/// Authority that controls a plugin once it has been added.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum PluginAuthority {
    None,
    Owner,
    UpdateAuthority,
    Address { address: Pubkey },
}

/// Add a frozen `FreezeDelegate` plugin to an asset and delegate thaw rights to
/// `freeze_authority` (the stake PDA). Adding an owner-managed plugin requires
/// the asset owner to sign as `authority`.
#[allow(clippy::too_many_arguments)]
pub fn freeze_asset<'info>(
    core_program: &AccountInfo<'info>,
    asset: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    freeze_authority: Pubkey,
) -> Result<()> {
    let mut data = vec![IX_ADD_PLUGIN];
    Plugin::FreezeDelegate(FreezeDelegate { frozen: true })
        .serialize(&mut data)
        .unwrap();
    Some(PluginAuthority::Address {
        address: freeze_authority,
    })
    .serialize(&mut data)
    .unwrap();

    let accounts = vec![
        AccountMeta::new(asset.key(), false),
        AccountMeta::new(collection.key(), false),
        AccountMeta::new(payer.key(), true),
        AccountMeta::new_readonly(authority.key(), true),
        AccountMeta::new_readonly(system_program.key(), false),
        AccountMeta::new_readonly(MPL_CORE_ID, false),
    ];
    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts,
        data,
    };
    invoke(
        &ix,
        &[
            asset.clone(),
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
    )?;
    Ok(())
}

/// Flip the `frozen` flag on an asset's `FreezeDelegate` plugin. The stake PDA
/// holds the plugin authority, so the call is signed with its seeds.
#[allow(clippy::too_many_arguments)]
pub fn set_frozen<'info>(
    core_program: &AccountInfo<'info>,
    asset: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    frozen: bool,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = vec![IX_UPDATE_PLUGIN];
    Plugin::FreezeDelegate(FreezeDelegate { frozen })
        .serialize(&mut data)
        .unwrap();

    let accounts = vec![
        AccountMeta::new(asset.key(), false),
        AccountMeta::new(collection.key(), false),
        AccountMeta::new(payer.key(), true),
        AccountMeta::new_readonly(authority.key(), true),
        AccountMeta::new_readonly(system_program.key(), false),
        AccountMeta::new_readonly(MPL_CORE_ID, false),
    ];
    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts,
        data,
    };
    invoke_signed(
        &ix,
        &[
            asset.clone(),
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
        signer_seeds,
    )?;
    Ok(())
}

/// Remove the `FreezeDelegate` plugin from an asset, signed by the stake PDA.
/// The asset must be thawed first.
pub fn remove_freeze<'info>(
    core_program: &AccountInfo<'info>,
    asset: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = vec![IX_REMOVE_PLUGIN];
    PluginType::FreezeDelegate.serialize(&mut data).unwrap();

    let accounts = vec![
        AccountMeta::new(asset.key(), false),
        AccountMeta::new(collection.key(), false),
        AccountMeta::new(payer.key(), true),
        AccountMeta::new_readonly(authority.key(), true),
        AccountMeta::new_readonly(system_program.key(), false),
        AccountMeta::new_readonly(MPL_CORE_ID, false),
    ];
    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts,
        data,
    };
    invoke_signed(
        &ix,
        &[
            asset.clone(),
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
        signer_seeds,
    )?;
    Ok(())
}

/// Add an `Attributes` plugin to a collection. Adding a collection plugin
/// requires the collection's update authority to sign.
pub fn add_collection_attributes<'info>(
    core_program: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    attribute_list: Vec<Attribute>,
    plugin_authority: Pubkey,
) -> Result<()> {
    let mut data = vec![IX_ADD_COLLECTION_PLUGIN];
    Plugin::Attributes(Attributes { attribute_list })
        .serialize(&mut data)
        .unwrap();
    Some(PluginAuthority::Address {
        address: plugin_authority,
    })
    .serialize(&mut data)
    .unwrap();

    let accounts = vec![
        AccountMeta::new(collection.key(), false),
        AccountMeta::new(payer.key(), true),
        AccountMeta::new_readonly(update_authority.key(), true),
        AccountMeta::new_readonly(system_program.key(), false),
        AccountMeta::new_readonly(MPL_CORE_ID, false),
    ];
    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts,
        data,
    };
    invoke(
        &ix,
        &[
            collection.clone(),
            payer.clone(),
            update_authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
    )?;
    Ok(())
}

/// Overwrite a collection's `Attributes` plugin. The config PDA holds the plugin
/// authority, so the call is signed with its seeds.
pub fn update_collection_attributes<'info>(
    core_program: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    attribute_list: Vec<Attribute>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = vec![IX_UPDATE_COLLECTION_PLUGIN];
    Plugin::Attributes(Attributes { attribute_list })
        .serialize(&mut data)
        .unwrap();

    let accounts = vec![
        AccountMeta::new(collection.key(), false),
        AccountMeta::new(payer.key(), true),
        AccountMeta::new_readonly(authority.key(), true),
        AccountMeta::new_readonly(system_program.key(), false),
        AccountMeta::new_readonly(MPL_CORE_ID, false),
    ];
    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts,
        data,
    };
    invoke_signed(
        &ix,
        &[
            collection.clone(),
            payer.clone(),
            authority.clone(),
            system_program.clone(),
            core_program.clone(),
        ],
        signer_seeds,
    )?;
    Ok(())
}
