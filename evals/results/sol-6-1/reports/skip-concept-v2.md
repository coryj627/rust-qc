Rust borrowing means **letting someone temporarily use something you own without giving it away**.

Imagine you own a notebook:

- **Several people can read it together.** Since nobody is changing anything, they won’t interfere with one another.
- **Someone who wants to edit it needs exclusive access.** Until they finish, nobody else can read or edit it. That prevents people from seeing half-finished changes or overwriting each other’s work.
- **You must keep the notebook available while it’s borrowed.** You can’t throw it away while someone is still using it.

Rust applies these rules to information inside a program. It checks that each loan is safe before the program runs, helping prevent mistakes such as changing information while someone else relies on it, or using information that no longer exists.