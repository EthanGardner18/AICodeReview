1def greet(name):
2    print(f"Hello, {name}!")
3
4def add_numbers(a, b):
5    return a + b
6
7def multiply_numbers(a, b):
8    return a * b
9
10def divide_numbers(a, b):
11    if b == 0:
12        return "Cannot divide by zero"
13    return a / b
14
15def find_max(numbers):
16    return max(numbers)
17
18def find_min(numbers):
19    return min(numbers)
20
21def calculate_average(numbers):
22    return sum(numbers) / len(numbers)
23
24def reverse_string(s):
25    return s[::-1]
26
27def is_palindrome(s):
28    s = s.lower().replace(" ", "")
29    return s == s[::-1]
30
31def count_vowels(s):
32    vowels = "aeiou"
33    return sum(1 for char in s.lower() if char in vowels)
34
35def factorial(n):
36    if n == 0:
37        return 1
38    return n * factorial(n - 1)
39
40def fibonacci(n):
41    if n <= 0:
42        return []
43    elif n == 1:
44        return [0]
45    elif n == 2:
46        return [0, 1]
47    fib = [0, 1]
48    for i in range(2, n):
49        fib.append(fib[-1] + fib[-2])
50    return fib
51
52def is_prime(n):
53    if n <= 1:
54        return False
55    for i in range(2, int(n**0.5) + 1):
56        if n % i == 0:
57            return False
58    return True
59
60def get_primes_up_to(n):
61    return [i for i in range(2, n + 1) if is_prime(i)]
62
63def convert_to_uppercase(s):
64    return s.upper()
65
66def convert_to_lowercase(s):
67    return s.lower()
68
69def remove_whitespace(s):
70    return s.strip()
71
72def calculate_area_of_circle(radius):
73    import math
74    return math.pi * (radius ** 2)
75
76def calculate_perimeter_of_rectangle(length, width):
77    return 2 * (length + width)
78
79def calculate_factorial_iterative(n):
80    result = 1
81    for i in range(1, n + 1):
82        result *= i
83    return result
84
85def find_unique_elements(elements):
86    return list(set(elements))
87
88def main():
89    greet("Riwaz")
90    print(add_numbers(5, 10))
91    print(multiply_numbers(2, 3))
92    print(divide_numbers(10, 2))
93    print(find_max([1, 2, 3, 4, 5]))
94    print(find_min([1, 2, 3, 4, 5]))
95    print(calculate_average([1, 2, 3, 4, 5]))
96    print(reverse_string("Python"))
97    print(is_palindrome("madam"))
98    print(count_vowels("hello"))
99    print(factorial(5))
100    print(fibonacci(10))
101    print(is_prime(17))
102    print(get_primes_up_to(20))
103    print(convert_to_uppercase("hello"))
104    print(convert_to_lowercase("HELLO"))
105    print(remove_whitespace("   hello   "))
106    print(calculate_area_of_circle(5))
107    print(calculate_perimeter_of_rectangle(4, 6))
108    print(calculate_factorial_iterative(5))
109    print(find_unique_elements([1, 2, 2, 3, 4, 4, 5]))
110
111if __name__ == "__main__":
112    main()
