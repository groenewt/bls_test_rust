# LINKS



## Drive folder

https://drive.google.com/drive/folders/14F65woWsNJI-V8CvM25cdCZcObTW7crH?usp=sharing





### DEALING WITH TAR

#### 🔁 **How to Extract (`untar`) the Archive**

For `.tar.xz`:

```bash
tar -I xz -xvf bls.tar.xz
```

or simply:

```bash
tar -xvf bls.tar.xz
```

(Tar auto-detects compression in most distros)

##### To extract to a specific directory:

```bash
tar -xvf bls.tar.xz -C /path/to/destination
```

---

#### 📌 Tip: Show Contents Without Extracting

```bash
tar -tvf bls.tar.xz
```

---

### Tar balls


```bash
tar -I 'xz -9e' -cvf bls.tar.xz ./raw/bls
```

