# Migration tasks

## Before the migration

- [x] Export the list of shares
- [x] Inform all users by e-mail
- [ ] Remove shares that nobody used in the last year
- [ ] Check that every share has an owner

## During the migration

- [ ] Freeze changes on the old server
- [ ] Run `ctsync migrate --source \\oldserver\shares`
  - [ ] Watch the progress in the admin center
  - [ ] Note any files that fail
- [ ] Compare the file counts

## After the migration

1. [ ] Turn off the old server after two weeks
2. [ ] Archive the migration log `migrate.log`
3. [x] Celebrate
