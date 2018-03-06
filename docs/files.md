## Sending project files to the devices

Some tests are relying on the presence of files at relative paths to be able
to proceed. But we can not always control where we will be executing from (we
can not always do `cd someplace` before running the tests).

So, the tests are "bundled" in the following way:

* root dinghy test directory
    * test_executable
    * recursive copy of the not-ignorable files and directories from your projects
    * test_data is contains configurable data to be sent to the device
        * some_file
        * some_dir

Anything in .gitignore or .dinghyignore will not be bundled.

To open your test file easily, you can use the dinghy-test crate in your tests which contains a helper function to access your project directory:

```rust
#[cfg(test)]
extern crate dinghy_test;


#[cfg(test)]
mod tests {
    #[test]
    fn my_test() {
        let my_file_path = dinghy_test::test_project_path().join(("tests/data_1.txt");
        // ...
    }
}
```

## Sending more files to the devices

Now let's assume you have out-of-repository files to send over. You can do that
by adding it in `.dinghy.toml` (you'll probably want this one in the project
directory, or just above it if the data is shared among several cargo projects).

```toml
[test_data]
the_data = "../data-2017-02-05"
conf_file = "/etc/some/file"
```

The keys are the name under which to look for files below "test_data" in the
bundles, and the values are what to be copied (from your development workstation).

By default anything in `.gitignore` or `.dinghyignore` is not copied, however if
you need .gitignore'd files to be copied it can be excluded by adding
`copy_git_ignored = true`:

```toml
[test_data]
the_data = { source = "../data-2017-02-05", copy_git_ignored = true }
conf_file = "/etc/some/file"
```

Then you can use again the dinghy-test crate to access your specific test data directory:

```rust
#[cfg(test)]
extern crate dinghy_test;


#[cfg(test)]
mod tests {
    #[test]
    fn my_test() {
        let my_test_data_path = dinghy_test::test_file_path("the_data");
        let my_test_file_path = dinghy_test::test_file_path("conf_file");
        // ...
    }
}
```


