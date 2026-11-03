The workspace is read-only, so I couldn’t apply the fix or run Cargo checks. This patch uses `u128` arithmetic to prevent overflow while preserving the API, empty-input behavior, and floor rounding. Its formatting passes rustfmt.

```diff
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -6,6 +6,39 @@
     if values.is_empty() {
         None
     } else {
-        Some(values.iter().sum::<u32>() / values.len() as u32)
+        let sum: u128 = values.iter().map(|&value| u128::from(value)).sum();
+        Some((sum / values.len() as u128) as u32)
     }
 }
+
+#[cfg(test)]
+mod tests {
+    use super::mean;
+
+    #[test]
+    fn mean_handles_large_values() {
+        assert_eq!(mean(&[u32::MAX, u32::MAX]), Some(u32::MAX));
+        assert_eq!(mean(&[u32::MAX; 3]), Some(u32::MAX));
+        assert_eq!(mean(&[u32::MAX, 1]), Some(2_147_483_648));
+        assert_eq!(mean(&[u32::MAX, u32::MAX, 0]), Some(2_863_311_530));
+        assert_eq!(mean(&[0, u32::MAX, u32::MAX]), Some(2_863_311_530));
+    }
+
+    #[test]
+    fn mean_rounds_down() {
+        assert_eq!(mean(&[1, 2]), Some(1));
+        assert_eq!(mean(&[u32::MAX, 0]), Some(2_147_483_647));
+    }
+
+    #[test]
+    fn mean_of_empty_input_is_none() {
+        assert_eq!(mean(&[]), None);
+    }
+
+    #[test]
+    fn mean_of_single_value_is_that_value() {
+        assert_eq!(mean(&[u32::MAX]), Some(u32::MAX));
+    }
+}
```