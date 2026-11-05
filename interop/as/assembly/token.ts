// The standard AssemblyScript MRC20 (mintable, burnable, ownable) with an
// argument-taking constructor, matching the Rust `rs_token` fixture.
import { Args } from '@massalabs/as-types';
import { mrc20Constructor } from '@massalabs/sc-standards/assembly/contracts/MRC20/MRC20';

export function constructor(bin: StaticArray<u8>): void {
  const a = new Args(bin);
  const name = a.nextString().expect('name');
  const symbol = a.nextString().expect('symbol');
  const decimals = a.nextU8().expect('decimals');
  const supply = a.nextU256().expect('totalSupply');
  mrc20Constructor(name, symbol, decimals, supply);
}

export {
  version,
  name,
  symbol,
  totalSupply,
  decimals,
  balanceOf,
  transfer,
  allowance,
  increaseAllowance,
  decreaseAllowance,
  transferFrom,
} from '@massalabs/sc-standards/assembly/contracts/MRC20/MRC20';
export { mint } from '@massalabs/sc-standards/assembly/contracts/MRC20/mintable/mint';
export { burn, burnFrom } from '@massalabs/sc-standards/assembly/contracts/MRC20/burnable/burn';
export { setOwner, ownerAddress, isOwner } from '@massalabs/sc-standards/assembly/contracts/utils/ownership';
