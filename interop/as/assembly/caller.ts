// Cross-contract call fixture: an AssemblyScript contract calling into other
// (Rust or AssemblyScript) contracts, generically and through the standard
// MRC20 wrapper of @massalabs/sc-standards.
import {
  Address,
  Context,
  Storage,
  call,
  functionExists,
  generateEvent,
  localCall,
  transferCoins,
} from '@massalabs/massa-as-sdk';
import { Args, boolToByte, bytesToString, stringToBytes, u256ToBytes, u64ToBytes } from '@massalabs/as-types';
import { MRC20Wrapper } from '@massalabs/sc-standards/assembly/contracts/MRC20/wrapper';
import { getBalanceEntryCost } from '@massalabs/sc-standards/assembly/contracts/MRC20/MRC20-external';
import { u256 } from 'as-bignum/assembly';

// forward(target: string, function: string, args: bytes, coins: u64) -> whatever the callee returns
export function forward(bin: StaticArray<u8>): StaticArray<u8> {
  const a = new Args(bin);
  const target = new Address(a.nextString().expect('target'));
  const func = a.nextString().expect('function');
  const args = a.nextBytes().expect('args');
  const coins = a.nextU64().expect('coins');
  return call(target, func, new Args(args), coins);
}

// forwardLocal(target: string, function: string, args: bytes): run the callee's code in this context
export function forwardLocal(bin: StaticArray<u8>): StaticArray<u8> {
  const a = new Args(bin);
  const target = new Address(a.nextString().expect('target'));
  const func = a.nextString().expect('function');
  const args = a.nextBytes().expect('args');
  return localCall(target, func, new Args(args));
}

// exists(target: string, function: string) -> bool
export function exists(bin: StaticArray<u8>): StaticArray<u8> {
  const a = new Args(bin);
  const target = new Address(a.nextString().expect('target'));
  const func = a.nextString().expect('function');
  return boolToByte(functionExists(target, func));
}

// tokenInfo(token: string) -> Args(name, symbol, decimals, totalSupply)
export function tokenInfo(bin: StaticArray<u8>): StaticArray<u8> {
  const token = new MRC20Wrapper(new Address(new Args(bin).nextString().expect('token')));
  return new Args()
    .add(token.name())
    .add(token.symbol())
    .add(token.decimals())
    .add(token.totalSupply())
    .serialize();
}

// tokenBalance(token: string, account: string) -> u256
export function tokenBalance(bin: StaticArray<u8>): StaticArray<u8> {
  const a = new Args(bin);
  const token = new MRC20Wrapper(new Address(a.nextString().expect('token')));
  return u256ToBytes(token.balanceOf(new Address(a.nextString().expect('account'))));
}

// tokenTransfer(token: string, to: string, amount: u256, coins: u64): transfer this contract's tokens
export function tokenTransfer(bin: StaticArray<u8>): void {
  const a = new Args(bin);
  const token = new MRC20Wrapper(new Address(a.nextString().expect('token')));
  const to = new Address(a.nextString().expect('to'));
  const amount = a.nextU256().expect('amount');
  const coins = a.nextU64().expect('coins');
  token.transfer(to, amount, coins);
  generateEvent(`as-caller transferred ${amount.toString()} to ${to.toString()}`);
}

// tokenApproveAndPull(token: string, owner: string, amount: u256, coins: u64): transferFrom owner to this contract
export function tokenPull(bin: StaticArray<u8>): void {
  const a = new Args(bin);
  const token = new MRC20Wrapper(new Address(a.nextString().expect('token')));
  const owner = new Address(a.nextString().expect('owner'));
  const amount = a.nextU256().expect('amount');
  const coins = a.nextU64().expect('coins');
  token.transferFrom(owner, Context.callee(), amount, coins);
}

// tokenAllowance(token: string, owner: string, spender: string) -> u256
export function tokenAllowance(bin: StaticArray<u8>): StaticArray<u8> {
  const a = new Args(bin);
  const token = new MRC20Wrapper(new Address(a.nextString().expect('token')));
  const owner = new Address(a.nextString().expect('owner'));
  const spender = new Address(a.nextString().expect('spender'));
  return u256ToBytes(token.allowance(owner, spender));
}

// tokenEntryCost(token: string, account: string) -> u64: storage a caller must
// fund for a fresh balance entry of `account` in `token` (reads the token's datastore).
export function tokenEntryCost(bin: StaticArray<u8>): StaticArray<u8> {
  const a = new Args(bin);
  const token = a.nextString().expect('token');
  const account = a.nextString().expect('account');
  return u64ToBytes(getBalanceEntryCost(token, account));
}

// Targets for a Rust caller.
export function ping(bin: StaticArray<u8>): StaticArray<u8> {
  const who = new Args(bin).nextString().expect('who');
  generateEvent(`as-caller pinged by ${Context.caller().toString()} as ${who}`);
  return stringToBytes(`pong ${who}`);
}

export function paid(_: StaticArray<u8>): StaticArray<u8> {
  Storage.set('paid', Context.transferredCoins().toString());
  return u64ToBytes(Context.transferredCoins());
}

export function whoCalls(_: StaticArray<u8>): StaticArray<u8> {
  return stringToBytes(Context.caller().toString());
}
