The workspace is read-only, so no files were changed. This patch passed all four unittest cases in memory. Rust was untouched.

```diff
--- a/greeting.py
+++ b/greeting.py
@@ -1,2 +1,3 @@
 def greeting(name):
-    return f"Hello, {name}!"
+    name = name.strip()
+    return f"Hello, {name or 'stranger'}!"
--- /dev/null
+++ b/test_greeting.py
@@ -0,0 +1,25 @@
+import unittest
+
+from greeting import greeting
+
+
+class GreetingTests(unittest.TestCase):
+    def test_named_greeting(self):
+        self.assertEqual(greeting("Alice"), "Hello, Alice!")
+
+    def test_strips_surrounding_whitespace(self):
+        self.assertEqual(greeting(" \tAlice\n "), "Hello, Alice!")
+
+    def test_preserves_internal_whitespace(self):
+        self.assertEqual(greeting("  Mary Jane  "), "Hello, Mary Jane!")
+
+    def test_empty_or_whitespace_name(self):
+        for name in ("", "   ", "\t\n\r"):
+            with self.subTest(name=name):
+                self.assertEqual(greeting(name), "Hello, stranger!")
+
+
+if __name__ == "__main__":
+    unittest.main()
+
+
```

After applying, run `python3 -m unittest discover -v` from `/work/crate`.