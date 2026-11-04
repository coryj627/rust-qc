import unittest

from greeting import greeting


class GreetingTests(unittest.TestCase):
    def test_greets_name(self):
        self.assertEqual(greeting("Alice"), "Hello, Alice!")

    def test_strips_surrounding_whitespace(self):
        self.assertEqual(greeting(" \t\nAlice \r\n"), "Hello, Alice!")

    def test_preserves_internal_whitespace(self):
        self.assertEqual(greeting("  Alice Smith  "), "Hello, Alice Smith!")

    def test_empty_or_whitespace_name_greets_stranger(self):
        for name in ("", "   ", "\t\r\n"):
            with self.subTest(name=name):
                self.assertEqual(greeting(name), "Hello, stranger!")


if __name__ == "__main__":
    unittest.main()
