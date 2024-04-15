## <span style="color:cyan">File Search Rust</span>
<img src="./logo.png" width="150" alt="">

### Project was created as university project for the course "Programming in Rust".

<span style="color:cyan">**File Search Rust**</span> aims to develop an efficient file search system for the Windows operating system.
Traditional search methods provided by the OS often demonstrate insufficient performance, especially when dealing with large volumes of data.
In response to this, we propose the creation of our own search system, based on file system indexing and the use of a trie data structure to accelerate the search process.

### Functionality
- **File System Indexing:** The process begins with indexing all files on the directory. 
    This allows the creation of a hashmap containing information about the location of each file. 
    Then the hashmap is serialized and saved to a file.
- **File Search Using a only hashmap:** After indexing, the user can perform file searches based on exact file name.
    This process is done by reading the serialized hashmap and searching for the file name.
- **Prefix Search:** The user can enable prefix search for files based on prefix of the file name.
- <span style="color:lime">**Image Indexing**</span> Main feature of our project is indexing images. 
    We generate captions for every image in the directory and save them in a database.
- <span style="color:lime">**Image Search:**</span> User can search for images based on the caption of the image.

### Libs
- serde
- bincode
- dioxus
- tokio
- trie-rs
- rusqlite
- serde_json
- reqwest
- rust2vec
- ndarray

 __Only__ the most important libraries are listed here. For full list of dependencies check `Cargo.toml`.

### Technologies
- **Azure**
- **Computer Vision API**
- **Natural Language Processing**
- **Multithreading**
- **Trie Data Structure**
- **SQLite Database**
- **Rust Programming Language**
- **Asynchronous Programming**
- **Serialization**
- **Image Processing**

### Authors
- [Anton Horobets](https://github.com/ikvict07)
- [Bohdan Koval](https://github.com/bogda165/)