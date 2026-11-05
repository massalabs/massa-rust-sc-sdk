// The standard AssemblyScript MRC1155 (metadata, mintable with the `minter`
// role, burnable, role-based access control, ownable), matching the Rust
// `rs_multi` fixture.
import { Args } from '@massalabs/as-types';
import { Context } from '@massalabs/massa-as-sdk';
import { mrc1155Constructor } from '@massalabs/sc-standards/assembly/contracts/MRC1155/MRC1155';
import { grantRole } from '@massalabs/sc-standards/assembly/contracts/utils/accessControl';
import { MINTER_ROLE } from '@massalabs/sc-standards/assembly/contracts/MRC1155/mintable/mint';
import { _setBaseURI, _setURI } from '@massalabs/sc-standards/assembly/contracts/MRC1155/metadata/metadata-internal';
import { onlyOwner } from '@massalabs/sc-standards/assembly/contracts/utils/ownership';

export function constructor(bin: StaticArray<u8>): void {
  const uri = new Args(bin).nextString().expect('uri');
  mrc1155Constructor(uri);
  // The deployer is the owner at this point: give it the minter role.
  grantRole(new Args().add(MINTER_ROLE).add(Context.caller().toString()).serialize());
}

export function setTokenURI(bin: StaticArray<u8>): void {
  onlyOwner();
  const a = new Args(bin);
  const id = a.nextU256().expect('id');
  const uri = a.nextString().expect('uri');
  _setURI(id, uri);
}

export function setBaseURI(bin: StaticArray<u8>): void {
  onlyOwner();
  _setBaseURI(new Args(bin).nextString().expect('uri'));
}

export {
  balanceOf,
  balanceOfBatch,
  setApprovalForAll,
  isApprovedForAll,
  safeTransferFrom,
  safeBatchTransferFrom,
} from '@massalabs/sc-standards/assembly/contracts/MRC1155/MRC1155';
export { uri } from '@massalabs/sc-standards/assembly/contracts/MRC1155/metadata/metadata';
export { mint, mintBatch } from '@massalabs/sc-standards/assembly/contracts/MRC1155/mintable/mint';
export { burn, burnBatch } from '@massalabs/sc-standards/assembly/contracts/MRC1155/burnable/burn';
export { grantRole, revokeRole, hasRole, members, onlyRole } from '@massalabs/sc-standards/assembly/contracts/utils/accessControl';
export { setOwner, ownerAddress, isOwner } from '@massalabs/sc-standards/assembly/contracts/utils/ownership';
