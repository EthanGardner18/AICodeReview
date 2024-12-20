// fibonacci.pas
program Fibonacci;
var
  n, a, b, c, i: integer;
begin
  Write('Enter the number of terms: ');
  ReadLn(n);
  a := 0;
  b := 1;
  Write(a, ' ', b, ' ');
  for i := 3 to n do
  begin
    c := a + b;
    Write(c, ' ');
    a := b;
    b := c;
  end;
  WriteLn;
end.

