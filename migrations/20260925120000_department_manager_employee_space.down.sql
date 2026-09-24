-- Irreversible in general (the user id is gone once converted). The down is
-- a no-op by design; re-deriving user ids from employee links cannot
-- distinguish multiple users per employee.
SELECT 1;
