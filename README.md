# KCD_Utils

KCD Utils is a command-line tool for modifying KISSEICOMTEC data files.

## Acquisition

Ensure that you have [Rust](https://www.rust-lang.org/tools/install) installed on your system. You can execute the crate via the command line using the following syntax:

## Installation

1. Clone the repository:

    ```bash
    git clone https://github.com/lycantrope/kcd_utils.git
    ```

2. Install `kcd_utils`:

    ```bash
    cd kcd_utils
    cargo install --path .
    ```
3. [Alternative] Install the kcd_utils from github url.
    ```bash
    cargo install --git "https://github.com/lycantrope/kcd_utils.git"
    ```
    

## Usage

```bash
kcd_utils <COMMAND>
```

### Commands:

- **Kcd: Rename and Modify KCD File**
  ```bash
  kcd_utils kcd --input <KCD FILE> --output <HDR FILE> [--mode <MODE>]
  ```

  - `--input`: KCD input file.
  - `--source`: HDR file to make association.
  - `--mode`: Method to generate the KCD file (Default: Copy).

- **Raf: Modify RAF File**
  ```bash
  kcd_utils raf --input <RAF FILE> --kcd <KCD FILE>
  ```

  - `--input`: Specify the input RAF file.
  - `--kcd`: Specify the KCD file for association.

- **Hdr: Output HDR Files**
  ```bash
  kcd_utils hdr --input <HDR FILE> --label <LABEL>
  ```

  - `--input`: Specify the input HDR file.
  - `--label`: Specify the label for new HDR file.

- **Video: Move or Copy Videos**
  ```bash
  kcd_utils video --src <SOURCE HDR FILE> --dst <TARGET HDR FILE> [--mode <MODE>]
  ```

  - `--src`: Specify a HDR file, which should be placed in the video folder.
  - `--dst`: Target HDR file, which should be placed in a folder named as HDR file.
  - `--mode`: Method to move the video (Default: Copy).


- **Clone: Clone entire KCD file and Videos**
  ```bash
  kcd_utils clone --src <SOURCE HDR FILE> --dst <TARGET HDR FILE> [--mode <MODE>]
  ```

  - `--src`: Specify the input KCD file.
  - `--label`: Specify the label for cloned KCD, HDR and video files.
  - `--mode`: Method to move the video (Default: Copy).


- **Help: Print Help**
  ```bash
  kcd_utils help
  ```

For more information about each command and its options, you can use the `help` command:

```bash
kcd_utils help <COMMAND>
```

## License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
