;; quicksort.lisp
(defun quicksort (list)
  (if (endp list)
      nil
      (let ((pivot (car list)))
        (append (quicksort (remove-if-not (lambda (x) (< x pivot)) (cdr list)))
                (list pivot)
                (quicksort (remove-if-not (lambda (x) (>= x pivot)) (cdr list)))))))

(print (quicksort '(3 1 4 1 5 9 2 6 5 3 5)))

