# ans_flex

ans_flex is a FSE/ANS implementation in Rust, a compressor in the family of entropy encoders (statistical compression).

FSE ([Finite State Entropy](https://github.com/Cyan4973/FiniteStateEntropy/)) is a ANS variant from Yann Collet. Main advantage is, that it requires only additions,
masks, and shifts.

ANS (Asymetric Numeral Systems) was introduced by Jarek Duda and is a popular compression standard
used in compression algorithms like zstd, due to its high compression ration and reasonable
compression speed. In comparison to huffman it has the advantage to using fractional bits, when encoding symbols.

If you want a better understanding of ANS, I can recommend "Understanding Compression" by Colton
McAnlis and Aleks Haecky as the foundation and then diving into the blog posts of [Charles Bloom](http://cbloomrants.blogspot.com/2014/01/1-30-14-understanding-ans-1.html)
and [Yann Collet](https://fastcompression.blogspot.com/2013/12/finite-state-entropy-new-breed-of.html)
The [ANS paper](https://arxiv.org/pdf/1311.2540.pdf) from Jarek Duda is also interesting, but without a solid
foundation in math and compression it will be difficult to follow.

Note, that entropy compression like ans is usually not purely used own its own, but in conjuncation with other compression techniques like Lempel-Ziv.

# Performance

Performance seems to be slightly faster than https://github.com/Cyan4973/FiniteStateEntropy/, which uses a close variant of its fse in zstd.

`cargo bench`

```

compression/ans_flex/66675                                                                            
                        time:   [176.11 us 176.37 us 176.66 us]
                        thrpt:  [359.93 MiB/s 360.52 MiB/s 361.05 MiB/s]
                 change:
                        time:   [-12.125% -11.822% -11.525%] (p = 0.00 < 0.05)
                        thrpt:  [+13.026% +13.407% +13.798%]
                        Performance has improved.
Found 5 outliers among 100 measurements (5.00%)
  5 (5.00%) high mild

decompression/deans_flex_reuse/66675                                                                            
                        time:   [110.24 us 110.95 us 111.84 us]
                        thrpt:  [568.53 MiB/s 573.08 MiB/s 576.80 MiB/s]
                 change:
                        time:   [-4.0021% -3.4458% -2.9494%] (p = 0.00 < 0.05)
                        thrpt:  [+3.0390% +3.5688% +4.1689%]
                        Performance has improved.
Found 5 outliers among 100 measurements (5.00%)
  3 (3.00%) high mild
  2 (2.00%) high severe


➜  programs git:(dev) ✗ ./fse -e -b ../../ans_flex/benches/compression_66k_JSON.txt
FSE : Finite State Entropy, 64-bits demo by Yann Collet (Feb 19 2021)
sion_66k_JSON.txt :     66675 ->     43661 (65.48%),  366.6 MB/s ,  469.9 MB/s


https://github.com/Cyan4973/FiniteStateEntropy/

```
