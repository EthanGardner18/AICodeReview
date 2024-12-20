! matrix_multiply.f90
program matrix_multiplication
    integer, parameter :: n = 3
    integer :: A(n, n), B(n, n), C(n, n)
    integer :: i, j, k

    ! Initialize matrices
    A = reshape([1, 2, 3, 4, 5, 6, 7, 8, 9], [n, n])
    B = reshape([9, 8, 7, 6, 5, 4, 3, 2, 1], [n, n])

    ! Matrix multiplication
    C = 0
    do i = 1, n
        do j = 1, n
            do k = 1, n
                C(i, j) = C(i, j) + A(i, k) * B(k, j)
            end do
        end do
    end do

    ! Output result
    print *, "Matrix A:"
    print *, A
    print *, "Matrix B:"
    print *, B
    print *, "Result of A * B:"
    print *, C
end program matrix_multiplication

