#include "foreign.h"

int binary_search(int arr[], int l, int r, int v)
{
    while (l <= r)
    {
        int m = l + (r - l) / 2;

        if (arr[m] == v)
        {
            return m;
        }

        if (arr[m] < v)
        {
            l = m + 1;
        }

        else
        {
            r = m - 1;
        }
    }

    return -1;
}

int zkmain()
{
    int x = read_public_input();

    int arr[] = {0, 1, 2, 3, 4, 5};
    int n = sizeof(arr) / sizeof(arr[0]);
    int result = binary_search(arr, 0, n - 1, x);

    return result;
}