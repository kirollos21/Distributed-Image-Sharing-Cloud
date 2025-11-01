# START HERE - Ultra Simple Guide

## 🎯 All You Need to Know

### Two Terminals, Two Commands:

#### Terminal 1: Start Server
```bash
cd /home/khoulykid@auc.egy/Desktop/Distributed-Image-Sharing-Cloud
cargo run --release --bin cloud-node -- 1 10.40.59.43:8001
```

#### Terminal 2: Run Test
```bash
cd /home/khoulykid@auc.egy/Desktop/Distributed-Image-Sharing-Cloud/single_img_testing
./simple_run.sh 10.40.59.43:8001
```

**That's it!** 

## 🖼️ What You'll See

3 image windows will open automatically:

1. **Original Image** - Clear, readable
2. **Encrypted Image** - Scrambled static/noise  
3. **Decrypted Image** - Clear again (proves it works!)

## 📁 Where Are The Files?

```bash
ls client_output/
```

You'll find:
- `01_original_image.jpg` - Copy of original
- `02_encrypted_image.jpg` - Encrypted (scrambled)
- `03_decrypted_image.png` - Decrypted (restored)

## ✅ How Do I Know It Worked?

You'll see:
- ✅ Terminal says "TEST COMPLETE - SUMMARY"
- ✅ 3 image windows opened
- ✅ Files created in `client_output/`
- ✅ Decrypted image looks identical to original

## 🔄 Run Again?

```bash
# Just run the same command
./simple_run.sh 10.40.59.43:8001
```

## 📖 More Details?

- **Quick Commands**: See `COMPLETE_GUIDE.md`
- **Full Documentation**: See `README.md`
- **Troubleshooting**: See `QUICK_REFERENCE.md`

---

**That's all you need! Just two commands and watch the magic happen! 🎉**
