// Args serialization fixture.
//
// `roundtrip` decodes every type `@massalabs/as-types` Args supports, in a
// fixed order, and re-encodes it: a byte-identical result proves the Rust
// encoder produced exactly what AssemblyScript reads, and vice versa.
// `golden` returns AssemblyScript-produced bytes for the Rust decoder.
// `ret*` return values with the usual AssemblyScript idioms.
import { Address } from '@massalabs/massa-as-sdk';
import {
  Args,
  Result,
  Serializable,
  boolToByte,
  stringToBytes,
  u256ToBytes,
  u64ToBytes,
} from '@massalabs/as-types';
import { i128, u128, u256 } from 'as-bignum/assembly';

export class Pair implements Serializable {
  constructor(public key: string = '', public value: u64 = 0) {}

  serialize(): StaticArray<u8> {
    return new Args().add(this.key).add(this.value).serialize();
  }

  deserialize(data: StaticArray<u8>, offset: i32): Result<i32> {
    const args = new Args(data, offset);
    this.key = args.nextString().expect('Pair.key');
    this.value = args.nextU64().expect('Pair.value');
    return new Result(args.offset);
  }
}

export function roundtrip(bin: StaticArray<u8>): StaticArray<u8> {
  const a = new Args(bin);
  const out = new Args();
  out.add(a.nextBool().expect('bool'));
  out.add(a.nextU8().expect('u8'));
  out.add(a.nextU16().expect('u16'));
  out.add(a.nextI16().expect('i16'));
  out.add(a.nextU32().expect('u32'));
  out.add(a.nextI32().expect('i32'));
  out.add(a.nextU64().expect('u64'));
  out.add(a.nextI64().expect('i64'));
  out.add(a.nextF32().expect('f32'));
  out.add(a.nextF64().expect('f64'));
  out.add(a.nextU128().expect('u128'));
  out.add(a.nextI128().expect('i128'));
  out.add(a.nextU256().expect('u256'));
  out.add(a.nextString().expect('string'));
  out.add(a.nextBytes().expect('bytes'));
  out.add(a.nextStringArray().expect('string[]'));
  out.add(a.nextFixedSizeArray<u64>().expect('u64[]'));
  out.add(a.nextFixedSizeArray<u256>().expect('u256[]'));
  out.add(a.nextSerializable<Pair>().expect('Pair'));
  out.addSerializableObjectArray(a.nextSerializableObjectArray<Pair>().expect('Pair[]'));
  out.add(a.nextSerializable<Address>().expect('Address'));
  assert(a.offset == bin.length, 'trailing bytes after the last argument');
  return out.serialize();
}

export function golden(_: StaticArray<u8>): StaticArray<u8> {
  return new Args()
    .add(true)
    .add<u8>(200)
    .add<u16>(65000)
    .add<i16>(-2)
    .add<u32>(0xdeadbeef)
    .add<i32>(-123456)
    .add<u64>(u64.MAX_VALUE)
    .add<i64>(-1)
    .add<f32>(1.5)
    .add<f64>(-2.25)
    .add(new u128(0x1122334455667788, 0x99aabbccddeeff00))
    .add(i128.fromI64(-5))
    .add(new u256(1, 2, 3, 0xffffffffffffffff))
    .add('héllo wörld 😀')
    .add<StaticArray<u8>>([1, 2, 3])
    .add<Array<string>>(['a', 'bc', ''])
    .add<Array<u64>>([1, 2, 3])
    .add<Array<u256>>([u256.fromU64(10), u256.Max])
    .add(new Pair('p', 7))
    .addSerializableObjectArray<Pair>([new Pair('x', 1), new Pair('y', 2)])
    .add(new Address('AU12UBnqTHDQALpocVBnkPNy7y5CndUJQTLutaVDDFgMJcq5kQiKq'))
    .serialize();
}

export function retString(_: StaticArray<u8>): StaticArray<u8> {
  return stringToBytes('héllo');
}

export function retU64(_: StaticArray<u8>): StaticArray<u8> {
  return u64ToBytes(42);
}

export function retBool(_: StaticArray<u8>): StaticArray<u8> {
  return boolToByte(true);
}

export function retU256(_: StaticArray<u8>): StaticArray<u8> {
  return u256ToBytes(u256.fromU64(7));
}

export function retArgs(_: StaticArray<u8>): StaticArray<u8> {
  return new Args().add('a').add<u32>(1).serialize();
}

export function retVoid(_: StaticArray<u8>): void {}

export function retEmpty(_: StaticArray<u8>): StaticArray<u8> {
  return [];
}

// Entry point without parameter, as some contracts declare them.
export function noParam(): StaticArray<u8> {
  return stringToBytes('ok');
}
