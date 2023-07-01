from z3 import *
from utils import *
from functools import reduce

s = init_z3_solver()

# define spec
lhs, rhs, res = Ints('lhs rhs res')
s.add(is_i64(lhs)),
s.add(is_i64(rhs)),
s.add(is_i64(res)),

wasm_mul_i64 = Function('WasmMulI64', IntSort(), IntSort(), IntSort())
s.add(ForAll([lhs, rhs], wasm_mul_i64(lhs, rhs) == (lhs * rhs) % I64_MODULUS))

# define var
aux = Int('aux')

constrain = Function('Constrain', IntSort(), BoolSort())
constraints = [
    is_i64(aux),
    # c.bin.mul
    fr_sub(fr_sub(fr_mul(rhs, lhs), fr_mul(aux, I64_MODULUS)), res) == 0
]

s.add(ForAll([aux], And(constrain(aux) == reduce(lambda x, y: And(x, y), constraints))))

s.push()
# Soundness
s.add(And(constrain(aux), wasm_mul_i64(lhs, rhs) != res))

check_res = s.check()
print('--------------Soundness---------------')
if check_res.r == Z3_L_TRUE:
    print('Verify: Fail')
    print(s.model())
elif check_res.r == Z3_L_FALSE:
    print('Verify: Pass')
else:
    print('Verify: Fail')
s.pop()


s.push()
# Completeness
s.add(And(wasm_mul_i64(lhs, rhs) == res, ForAll([aux], Not(constrain(aux)))))

check_res = s.check()
print('-------------Completeness----------------')
if check_res.r == Z3_L_TRUE:
    print('Verify: Fail')
    print(s.model())
elif check_res.r == Z3_L_FALSE:
    print('Verify: Pass')
else:
    print('Verify: Fail')
s.pop()
