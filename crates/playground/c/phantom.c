#include <stdint.h>
#include "foreign.h"

/*
 * An example of a phantom function that will not generate any traces.
 *
 * To make the phantom function work, you should make sure the function
 * not to be inlined.
 *
 * Since the phantom function will not produce any traces, memory/global
 * writing is invisible to prover,
 * *** the function MUST NOT have these operations ***.
 */
__attribute__((noinline)) int search(int *arr, int size, int v)
{
	for (int i = 0; i < size; i++)
	{
		if (arr[i] == v)
		{
			return i;
		}
	}

	return -1;
}

__attribute__((visibility("default"))) void zkmain()
{
	uint64_t v = read_public_input();
	int arr[] = {0, 1, 2, 3, 4};

	int pos = search(arr, 5, v);
	require(arr[pos] == v);
}
