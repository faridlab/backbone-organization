-- Migration: departments.manager_id moves to the employee id space
--
-- A department head was stored as a sapiens USER id while a direct manager
-- (employments.direct_manager_id) is an EMPLOYEE id — approver resolution
-- had to know which and convert. One space everywhere: employee ids. The
-- data migration resolves each user id through employee.employees.user_id
-- (the link the invite verb maintains); a head with no employee record is
-- nulled and reported, never guessed.

UPDATE organization.departments d
   SET manager_id = e.id
  FROM employee.employees e
 WHERE e.user_id = d.manager_id
   AND (e.metadata->>'deleted_at') IS NULL
   AND d.manager_id IS NOT NULL;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM organization.departments d
               WHERE d.manager_id IS NOT NULL
                 AND NOT EXISTS (SELECT 1 FROM employee.employees e
                                  WHERE e.id = d.manager_id
                                    AND (e.metadata->>'deleted_at') IS NULL)) THEN
        RAISE WARNING 'departments.manager_id: some heads did not resolve to employee records — nulled';
    END IF;
END $$;

-- Null the unresolvable (a user id left in an employee-id column is worse
-- than an absent head: it would resolve against the wrong id space).
UPDATE organization.departments d
   SET manager_id = NULL
 WHERE d.manager_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM employee.employees e
                    WHERE e.id = d.manager_id
                      AND (e.metadata->>'deleted_at') IS NULL);
