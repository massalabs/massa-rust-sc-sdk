// Datastore layout fixture: writes with every storage helper of the
// AssemblyScript SDK and reads everything back. The Rust `rs_storage` fixture
// does the same with the Rust helpers; the suite swaps the bytecode of one for
// the other on top of the same datastore and checks that both read identical
// values.
import {
  Address,
  ConstantManager,
  Context,
  KeyIncrementer,
  MapManager,
  Storage,
  collections,
  security,
} from '@massalabs/massa-as-sdk';
import {
  Args,
  Result,
  Serializable,
  boolToByte,
  bytesToF64,
  bytesToI64,
  bytesToU32,
  bytesToU64,
  stringToBytes,
  u32ToBytes,
  u64ToBytes,
} from '@massalabs/as-types';

const USER = 'AU12UBnqTHDQALpocVBnkPNy7y5CndUJQTLutaVDDFgMJcq5kQiKq';

class Item implements Serializable {
  constructor(public name: string = '', public qty: u32 = 0) {}

  serialize(): StaticArray<u8> {
    return new Args().add(this.name).add(this.qty).serialize();
  }

  deserialize(data: StaticArray<u8>, offset: i32 = 0): Result<i32> {
    const args = new Args(data, offset);
    this.name = args.nextString().expect('Item.name');
    this.qty = args.nextU32().expect('Item.qty');
    return new Result(args.offset);
  }
}

export function write(_: StaticArray<u8>): void {
  // Raw Storage with the three supported generic types and the usual
  // fixed-width helpers.
  Storage.set('str', 'hello');
  Storage.set<StaticArray<u8>>(stringToBytes('bytes'), [1, 2, 3]);
  Storage.set<Args>(new Args().add('k'), new Args().add<u32>(5).add('v'));
  Storage.set(stringToBytes('u64'), u64ToBytes(42));
  Storage.set(stringToBytes('u32'), u32ToBytes(7));
  Storage.set(stringToBytes('bool'), boolToByte(true));

  // PersistentMap. Primitive values can be written (as 8 bytes) but not read
  // back by the AssemblyScript class (`V | null` is invalid for primitives);
  // `read` below decodes them from the raw entries.
  new collections.PersistentMap<string, string>('ms').set('k', 'v');
  new collections.PersistentMap<string, u32>('mi').set('k', 7);
  new collections.PersistentMap<string, u64>('mu').set('big', u64.MAX_VALUE);
  new collections.PersistentMap<string, bool>('mb').set('k', true);
  new collections.PersistentMap<string, f64>('mf').set('k', 1.5);
  new collections.PersistentMap<Address, u64>('ma').set(new Address(USER), 9);
  new collections.PersistentMap<u32, StaticArray<u8>>('mk').set(12, [9, 8]);
  new collections.PersistentMap<string, Item>('mo').set('it', new Item('apple', 3));

  // Key managers.
  const seq = new KeyIncrementer<u8>(0);
  new ConstantManager<u64>(seq).set(123);
  new ConstantManager<string>(seq).set('constant');
  new MapManager<string, u32>(seq).set('x', 5);
  new MapManager<u64, string>(seq).set(77, 'seventy-seven');

  // Bitmask access control.
  const acl = new security.AccessControl<u8>(9);
  const ADMIN = acl.newPermission('admin');
  const USER_PERM = acl.newPermission('user');
  acl.grantPermission(ADMIN, new Address(USER));
  acl.grantPermission(USER_PERM, new Address(USER));
  acl.grantPermission(USER_PERM, Context.caller());
}

// Reads everything `write` stored and returns it as one Args, in a fixed order.
export function read(_: StaticArray<u8>): StaticArray<u8> {
  const out = new Args();
  out.add(Storage.get('str'));
  out.add(Storage.get<StaticArray<u8>>(stringToBytes('bytes')));
  const args = Storage.get<Args>(new Args().add('k'));
  out.add(args.nextU32().expect('u32')).add(args.nextString().expect('string'));
  out.add(bytesToU64(Storage.get(stringToBytes('u64'))));
  out.add(bytesToU32(Storage.get(stringToBytes('u32'))));
  out.add(Storage.get(stringToBytes('bool'))[0] == 1);

  out.add(new collections.PersistentMap<string, string>('ms').getSome('k').unwrap());
  out.add<u32>(<u32>bytesToI64(Storage.get(stringToBytes('mi::k'))));
  out.add<u64>(<u64>bytesToI64(Storage.get(stringToBytes('mu::big'))));
  out.add(Storage.get(stringToBytes('mb::k'))[0] == 1);
  out.add<f64>(bytesToF64(Storage.get(stringToBytes('mf::k'))));
  out.add<u64>(<u64>bytesToI64(Storage.get(stringToBytes('ma::' + USER))));
  out.add(new collections.PersistentMap<u32, StaticArray<u8>>('mk').getSome(12).unwrap());
  const item = new collections.PersistentMap<string, Item>('mo').get('it', new Item())!;
  out.add(item.name).add(item.qty);
  out.add(new collections.PersistentMap<string, string>('ms').contains('missing'));

  const seq = new KeyIncrementer<u8>(0);
  out.add(new ConstantManager<u64>(seq).mustValue());
  out.add(new ConstantManager<string>(seq).mustValue());
  out.add(new MapManager<string, u32>(seq).mustValue('x'));
  out.add(new MapManager<u64, string>(seq).mustValue(77));

  const acl = new security.AccessControl<u8>(9);
  const ADMIN = acl.newPermission('admin');
  const USER_PERM = acl.newPermission('user');
  out.add(acl.hasPermission(ADMIN, new Address(USER)));
  out.add(acl.hasPermission(USER_PERM, new Address(USER)));
  out.add(acl.hasPermission(ADMIN, Context.caller()));
  out.add(acl.hasPermission(USER_PERM, Context.caller()));
  return out.serialize();
}

// keys() -> Args(count: u32, key: bytes...): every datastore key, for layout comparison.
export function keys(_: StaticArray<u8>): StaticArray<u8> {
  const all = Storage.getKeys();
  const out = new Args();
  out.add<u32>(all.length);
  for (let i = 0; i < all.length; i++) {
    out.add(all[i]);
  }
  return out.serialize();
}
