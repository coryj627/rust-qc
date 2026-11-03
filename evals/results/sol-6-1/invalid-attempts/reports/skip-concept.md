Rust borrowing is a way to let part of a program use something without taking ownership of it.

Imagine you own a notebook. You can lend it out, but it still belongs to you. Rust applies two borrowing rules:

- **For reading:** Several people can look at the notebook together, as long as nobody changes it.
- **For making changes:** One person gets exclusive access. While they’re writing, nobody else can read or write in it.

That second rule prevents someone from reading a page while another person is erasing it.

Rust also checks that the notebook stays available for as long as anyone is borrowing it. Its owner cannot throw it away while someone still needs it.

These checks happen before the program runs. They help prevent mistakes caused by changing information unexpectedly or trying to use something that no longer exists.