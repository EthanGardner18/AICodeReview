       IDENTIFICATION DIVISION.
       PROGRAM-ID. payroll.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 employee-name PIC A(30).
       01 hours-worked PIC 9(3).
       01 hourly-rate PIC 9(3)V99.
       01 salary PIC 9(5)V99.

       PROCEDURE DIVISION.
       DISPLAY "Enter employee name: ".
       ACCEPT employee-name.
       DISPLAY "Enter hours worked: ".
       ACCEPT hours-worked.
       DISPLAY "Enter hourly rate: ".
       ACCEPT hourly-rate.

       COMPUTE salary = hours-worked * hourly-rate.
       DISPLAY "Employee: " employee-name.
       DISPLAY "Salary: $" salary.
       STOP RUN.

