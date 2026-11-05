// The standard AssemblyScript MRC721 (enumerable + metadata, ownable),
// matching the Rust `rs_nft` fixture.
import { Args } from '@massalabs/as-types';
import { mrc721Constructor } from '@massalabs/sc-standards/assembly/contracts/MRC721/enumerable/MRC721Enumerable';
import { _setBaseURI, _setURI } from '@massalabs/sc-standards/assembly/contracts/MRC721/metadata/metadata-internal';
import { onlyOwner } from '@massalabs/sc-standards/assembly/contracts/utils/ownership';

export function constructor(bin: StaticArray<u8>): void {
  const a = new Args(bin);
  const name = a.nextString().expect('name');
  const symbol = a.nextString().expect('symbol');
  const baseUri = a.nextString().expect('baseURI');
  mrc721Constructor(name, symbol);
  _setBaseURI(baseUri);
}

// setTokenURI(tokenId: u256, uri: string), owner only.
export function setTokenURI(bin: StaticArray<u8>): void {
  onlyOwner();
  const a = new Args(bin);
  const id = a.nextU256().expect('tokenId');
  const uri = a.nextString().expect('uri');
  _setURI(id, uri);
}

export {
  name,
  symbol,
  balanceOf,
  ownerOf,
  getApproved,
  isApprovedForAll,
  approve,
  setApprovalForAll,
  transferFrom,
  mint,
  burn,
  totalSupply,
  ownerAddress,
} from '@massalabs/sc-standards/assembly/contracts/MRC721/enumerable/MRC721Enumerable';
export { uri } from '@massalabs/sc-standards/assembly/contracts/MRC721/metadata/metadata';
export { setOwner, isOwner } from '@massalabs/sc-standards/assembly/contracts/utils/ownership';
