import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  mplCore,
  createCollection,
  create,
  fetchAsset,
  fetchCollection,
  type CollectionV1,
} from "@metaplex-foundation/mpl-core";
import { keypairIdentity, generateSigner, type Umi } from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { toWeb3JsPublicKey } from "@metaplex-foundation/umi-web3js-adapters";
import { assert } from "chai";

import { NftStaking } from "../target/types/nft_staking";

const MPL_CORE_PROGRAM = new PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"
);

const POINTS_PER_STAKE = 10; // reward points per second per staked asset
const MAX_STAKE = 3;
const FREEZE_PERIOD = 5; // seconds an asset must stay staked before unstaking

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const stakedCount = (collection: CollectionV1): string | undefined =>
  collection.attributes?.attributeList.find((a) => a.key === "staked")?.value;

describe("nft-staking", () => {
  const envProvider = anchor.AnchorProvider.env();
  const connection = envProvider.connection;

  // A dedicated, airdropped wallet acts as admin, staker, collection authority
  // and asset owner across both Anchor and umi, and pays every transaction.
  const wallet = new anchor.Wallet(Keypair.generate());
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  const program = new Program<NftStaking>(
    anchor.workspace.nftStaking.idl,
    provider
  );
  const owner = wallet.publicKey;

  let umi: Umi;
  let collectionPk: PublicKey;
  let assetPk: PublicKey;

  const [config] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );
  const [rewardsMint] = PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), config.toBuffer()],
    program.programId
  );
  const [userAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from("user"), owner.toBuffer()],
    program.programId
  );
  const rewardsAta = getAssociatedTokenAddressSync(rewardsMint, owner);

  let stakeAccount: PublicKey;

  before(async () => {
    const sig = await connection.requestAirdrop(owner, 100 * 1e9);
    const bh = await connection.getLatestBlockhash();
    await connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");

    umi = createUmi(connection.rpcEndpoint, "confirmed").use(mplCore());
    umi.use(keypairIdentity(umi.eddsa.createKeypairFromSecretKey(wallet.payer.secretKey)));

    // Mint a Core collection and one asset inside it, both owned by the wallet.
    const collectionSigner = generateSigner(umi);
    await createCollection(umi, {
      collection: collectionSigner,
      name: "Staking Collection",
      uri: "https://example.com/collection.json",
    }).sendAndConfirm(umi);
    const collection = await fetchCollection(umi, collectionSigner.publicKey);

    const assetSigner = generateSigner(umi);
    await create(umi, {
      asset: assetSigner,
      collection,
      name: "Staking NFT",
      uri: "https://example.com/asset.json",
    }).sendAndConfirm(umi);

    collectionPk = toWeb3JsPublicKey(collectionSigner.publicKey);
    assetPk = toWeb3JsPublicKey(assetSigner.publicKey);
    [stakeAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("stake"), assetPk.toBuffer(), config.toBuffer()],
      program.programId
    );
  });

  it("initializes the config and rewards mint", async () => {
    await program.methods
      .initializeConfig(POINTS_PER_STAKE, MAX_STAKE, new anchor.BN(FREEZE_PERIOD))
      .accountsPartial({
        admin: owner,
        config,
        rewardsMint,
        collection: collectionPk,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const cfg = await program.account.stakeConfig.fetch(config);
    assert.equal(cfg.pointsPerStake, POINTS_PER_STAKE);
    assert.equal(cfg.maxStake, MAX_STAKE);
    assert.equal(cfg.freezePeriod.toNumber(), FREEZE_PERIOD);
    assert.equal(cfg.stakedCount, 0);
    assert.ok(cfg.collection.equals(collectionPk));
  });

  it("adds an Attributes plugin to the collection", async () => {
    await program.methods
      .initializeCollection()
      .accountsPartial({
        payer: owner,
        config,
        collection: collectionPk,
        updateAuthority: owner,
        mplCoreProgram: MPL_CORE_PROGRAM,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const collection = await fetchCollection(umi, collectionPk.toBase58());
    assert.equal(stakedCount(collection), "0");
  });

  it("initializes a user", async () => {
    await program.methods
      .initializeUser()
      .accountsPartial({
        user: owner,
        userAccount,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const user = await program.account.userAccount.fetch(userAccount);
    assert.equal(user.amountStaked, 0);
    assert.equal(user.points.toNumber(), 0);
  });

  it("stakes an asset and freezes it", async () => {
    await program.methods
      .stake()
      .accountsPartial({
        owner,
        config,
        userAccount,
        stakeAccount,
        asset: assetPk,
        collection: collectionPk,
        mplCoreProgram: MPL_CORE_PROGRAM,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const asset = await fetchAsset(umi, assetPk.toBase58());
    assert.equal(asset.freezeDelegate?.frozen, true, "asset should be frozen");

    const user = await program.account.userAccount.fetch(userAccount);
    assert.equal(user.amountStaked, 1);

    const cfg = await program.account.stakeConfig.fetch(config);
    assert.equal(cfg.stakedCount, 1);

    const collection = await fetchCollection(umi, collectionPk.toBase58());
    assert.equal(stakedCount(collection), "1");
  });

  it("rejects unstaking before the freeze period elapses", async () => {
    try {
      await program.methods
        .unstake()
        .accountsPartial({
          owner,
          config,
          userAccount,
          stakeAccount,
          asset: assetPk,
          collection: collectionPk,
          mplCoreProgram: MPL_CORE_PROGRAM,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      assert.fail("unstake should have been rejected");
    } catch (err) {
      assert.include(err.toString(), "FreezePeriodNotElapsed");
    }

    // The asset stays frozen after a failed unstake.
    const asset = await fetchAsset(umi, assetPk.toBase58());
    assert.equal(asset.freezeDelegate?.frozen, true);
  });

  it("claims rewards without unstaking the asset", async () => {
    await sleep(2000); // let some reward points accrue

    await program.methods
      .claim()
      .accountsPartial({
        owner,
        config,
        userAccount,
        stakeAccount,
        rewardsMint,
        rewardsAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const balance = await connection.getTokenAccountBalance(rewardsAta);
    assert.ok(Number(balance.value.amount) > 0, "rewards should have been minted");

    const user = await program.account.userAccount.fetch(userAccount);
    assert.equal(user.points.toNumber(), 0, "points reset after claiming");
    assert.equal(user.amountStaked, 1, "asset is still staked");

    // Still frozen: claiming did not unstake.
    const asset = await fetchAsset(umi, assetPk.toBase58());
    assert.equal(asset.freezeDelegate?.frozen, true);
  });

  it("unstakes right after claiming and thaws the asset", async () => {
    await sleep(FREEZE_PERIOD * 1000); // wait out the rest of the freeze period

    const before = await connection.getTokenAccountBalance(rewardsAta);

    // Claim once more, then unstake immediately.
    await program.methods
      .claim()
      .accountsPartial({
        owner,
        config,
        userAccount,
        stakeAccount,
        rewardsMint,
        rewardsAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .unstake()
      .accountsPartial({
        owner,
        config,
        userAccount,
        stakeAccount,
        asset: assetPk,
        collection: collectionPk,
        mplCoreProgram: MPL_CORE_PROGRAM,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // Reward balance grew from the second claim.
    const after = await connection.getTokenAccountBalance(rewardsAta);
    assert.ok(Number(after.value.amount) > Number(before.value.amount));

    // The asset is thawed and the freeze plugin removed.
    const asset = await fetchAsset(umi, assetPk.toBase58());
    assert.isUndefined(asset.freezeDelegate, "freeze plugin should be removed");

    const user = await program.account.userAccount.fetch(userAccount);
    assert.equal(user.amountStaked, 0);

    const cfg = await program.account.stakeConfig.fetch(config);
    assert.equal(cfg.stakedCount, 0);

    const collection = await fetchCollection(umi, collectionPk.toBase58());
    assert.equal(stakedCount(collection), "0");

    // The stake record was closed.
    const closed = await connection.getAccountInfo(stakeAccount);
    assert.isNull(closed);
  });
});
