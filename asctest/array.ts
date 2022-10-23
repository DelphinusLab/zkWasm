export function test_array(): Array<i32> {
  return [42];
}

function test_new_array(): Array<i32> {
  var str_arr = new Array<string>(10);
  var int_arr = new Array<i32>(10);
  return str_arr;
}

function test_length_array(): i32 {
  var arr = [1,2,3]
  return arr.length;
}

function test_set_array(): void {
  var str_arr = new Array<string>(10);
  for (let i = 0; i < str_arr.length; ++i) {
    str_arr[i] = ""
  }
}
