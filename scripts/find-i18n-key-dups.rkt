;; Usage: run at the root of the repository

#lang racket

(define (parse-line line)
  (cond
    [(string-prefix? line "#") #f]
    [(null? (string-split line)) #f]
    [else (car (string-split line))]))

(define (find-duplicated-keys found lines)
  (unless (null? lines)
    (define key (parse-line (car lines)))
    (if key
        (cond
          [(set-member? found key)
           (displayln (format "duplicate: ~a" key))
           (find-duplicated-keys found (cdr lines))]
          [else (find-duplicated-keys (set-add found key) (cdr lines))])
        (find-duplicated-keys found (cdr lines)))))

(for ([p (directory-list "i18n")])
  (define path (build-path "i18n" p "oma.ftl"))
  (with-input-from-file path
                        (λ ()
                          (displayln path)
                          (find-duplicated-keys (set) (port->lines))
                          (newline))))
