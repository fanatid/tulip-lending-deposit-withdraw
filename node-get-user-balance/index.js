const web3 = require("@solana/web3.js");
const { Token, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } = require("@solana/spl-token");
const { publicKey, u64, u8, u128, u32, bool, struct } = require("@project-serum/borsh");
const BN = require("bn.js");

const LENDING_RESERVE_LAYOUT = struct([
  u8('version'),
  struct(
    [
      u64('slot'),
      bool('stale')
    ],
    'lastUpdateSlot'
  ),
  publicKey('lendingMarket'),
  publicKey('borrowAuthorizer'),
  struct(
    [
      publicKey('mintPubKey'),
      u8('mintDecimals'),
      publicKey('supplyPubKey'),
      publicKey('feeReceiver'),
      publicKey('oraclePubKey'),
      u64('availableAmount'),
      u128('borrowedAmount'),
      u128('cumulativeBorrowRate'),
      u128('marketPrice'),
      u128('platformAmountWads'),
      u8('platformFees')
    ],
    'liquidity'
  )
]);

;(async () => {
  const conn = new web3.Connection("https://solana-api.projectserum.com");

  // https://gist.github.com/therealssj/934c9b1d23a97f0d099b74abbdc31526
  const addressCollateralMint = new web3.PublicKey("Amig8TisuLpzun8XyGfC5HJHHGUQEscjLgoTWsCCKihg");
  const addressReserve = new web3.PublicKey("FTkSmGsJ3ZqDSHdcnY7ejN1pWV3Ej7i88MYpZyyaqgGt");
  const addressUser = new web3.PublicKey("FTqaWjTNTM35eWwxE64zmxzqZXFcLH5t7bvRqJCcTXWU");

  const tokenLending = new Token(conn, addressCollateralMint, TOKEN_PROGRAM_ID, web3.Keypair.generate());
  const addressUserAssociated = await Token.getAssociatedTokenAddress(
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    addressCollateralMint,
    addressUser
  );

  const userInfo = await tokenLending.getAccountInfo(addressUserAssociated);
  console.log(userInfo.amount.toString() / 10 ** 6);

  const collateralInfo = await tokenLending.getMintInfo(addressCollateralMint);
  console.log(collateralInfo.supply.toString() / 10 ** 6);

  const accReserve = await conn.getAccountInfo(addressReserve);
  const reserve = LENDING_RESERVE_LAYOUT.decode(accReserve.data);
  console.log(reserve);
  console.log(reserve.liquidity.availableAmount.toString() / 10 ** 6);
  console.log(reserve.liquidity.borrowedAmount.toString() / 10 ** 18 / 10 ** 6);
  console.log(reserve.liquidity.platformAmountWads.toString() / 10 ** 18 / 10 ** 6);
  const wad = new BN(10).pow(new BN(18));
  const total = reserve.liquidity.availableAmount
    .add(reserve.liquidity.borrowedAmount.div(wad))
    .sub(reserve.liquidity.platformAmountWads.div(wad));
  console.log(total.toString() / 10 ** 6);

  const balance = userInfo.amount.mul(total).div(collateralInfo.supply);
  console.log("balance:", balance.toString() / 10 ** 6);
})();
