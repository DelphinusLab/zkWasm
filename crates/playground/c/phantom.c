#include <stdint.h>
#include "foreign.h"

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
