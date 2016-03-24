use super::Future;
use std::fmt::Debug;

pub fn join2<A, B, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>
) -> Future<(A, B), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.map(|b| (a, b))
    })
}

pub fn join3<A, B, C, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>
) -> Future<(A, B, C), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.map(|c| (a,b,c))
        })
    })
}

pub fn join4<A, B, C, D, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>,
    fd: Future<D, ERR>,
) -> Future<(A, B, C, D), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          D: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.and_thenf(|c| {
                fd.map(|d| (a, b, c, d))
            })
        })
    })
}

pub fn join5<A, B, C, D, E, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>,
    fd: Future<D, ERR>,
    fe: Future<E, ERR>,
) -> Future<(A, B, C, D, E), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          D: Debug + 'static,
          E: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.and_thenf(|c| {
                fd.and_thenf(|d| {
                    fe.map(|e| (a, b, c, d, e))
                })
            })
        })
    })
}

pub fn join6<A, B, C, D, E, F, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>,
    fd: Future<D, ERR>,
    fe: Future<E, ERR>,
    ff: Future<F, ERR>,
) -> Future<(A, B, C, D, E, F), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          D: Debug + 'static,
          E: Debug + 'static,
          F: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.and_thenf(|c| {
                fd.and_thenf(|d| {
                    fe.and_thenf(|e| {
                        ff.map(|f| (a, b, c, d, e, f))
                    })
                })
            })
        })
    })
}

pub fn join7<A, B, C, D, E, F, G, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>,
    fd: Future<D, ERR>,
    fe: Future<E, ERR>,
    ff: Future<F, ERR>,
    fg: Future<G, ERR>,
) -> Future<(A, B, C, D, E, F, G), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          D: Debug + 'static,
          E: Debug + 'static,
          F: Debug + 'static,
          G: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.and_thenf(|c| {
                fd.and_thenf(|d| {
                    fe.and_thenf(|e| {
                        ff.and_thenf(|f| {
                            fg.map(|g| (a, b, c, d, e, f, g))
                        })
                    })
                })
            })
        })
    })
}

pub fn join8<A, B, C, D, E, F, G, H, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>,
    fd: Future<D, ERR>,
    fe: Future<E, ERR>,
    ff: Future<F, ERR>,
    fg: Future<G, ERR>,
    fh: Future<H, ERR>,
) -> Future<(A, B, C, D, E, F, G, H), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          D: Debug + 'static,
          E: Debug + 'static,
          F: Debug + 'static,
          G: Debug + 'static,
          H: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.and_thenf(|c| {
                fd.and_thenf(|d| {
                    fe.and_thenf(|e| {
                        ff.and_thenf(|f| {
                            fg.and_thenf(|g| {
                                fh.map(|h| (a, b, c, d, e, f, g, h))
                            })
                        })
                    })
                })
            })
        })
    })
}

pub fn join9<A, B, C, D, E, F, G, H, I, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>,
    fd: Future<D, ERR>,
    fe: Future<E, ERR>,
    ff: Future<F, ERR>,
    fg: Future<G, ERR>,
    fh: Future<H, ERR>,
    fi: Future<I, ERR>,
) -> Future<(A, B, C, D, E, F, G, H, I), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          D: Debug + 'static,
          E: Debug + 'static,
          F: Debug + 'static,
          G: Debug + 'static,
          H: Debug + 'static,
          I: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.and_thenf(|c| {
                fd.and_thenf(|d| {
                    fe.and_thenf(|e| {
                        ff.and_thenf(|f| {
                            fg.and_thenf(|g| {
                                fh.and_thenf(|h| {
                                    fi.map(|i| (a, b, c, d, e, f, g, h, i))
                                })
                            })
                        })
                    })
                })
            })
        })
    })
}

pub fn join10<A, B, C, D, E, F, G, H, I, J, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>,
    fd: Future<D, ERR>,
    fe: Future<E, ERR>,
    ff: Future<F, ERR>,
    fg: Future<G, ERR>,
    fh: Future<H, ERR>,
    fi: Future<I, ERR>,
    fj: Future<J, ERR>
) -> Future<(A, B, C, D, E, F, G, H, I, J), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          D: Debug + 'static,
          E: Debug + 'static,
          F: Debug + 'static,
          G: Debug + 'static,
          H: Debug + 'static,
          I: Debug + 'static,
          J: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.and_thenf(|c| {
                fd.and_thenf(|d| {
                    fe.and_thenf(|e| {
                        ff.and_thenf(|f| {
                            fg.and_thenf(|g| {
                                fh.and_thenf(|h| {
                                    fi.and_thenf(|i| {
                                        fj.map(|j| (a, b, c, d, e, f, g, h, i, j))
                                    })
                                })
                            })
                        })
                    })
                })
            })
        })
    })
}

pub fn join11<A, B, C, D, E, F, G, H, I, J, K, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>,
    fd: Future<D, ERR>,
    fe: Future<E, ERR>,
    ff: Future<F, ERR>,
    fg: Future<G, ERR>,
    fh: Future<H, ERR>,
    fi: Future<I, ERR>,
    fj: Future<J, ERR>,
    fk: Future<K, ERR>
) -> Future<(A, B, C, D, E, F, G, H, I, J, K), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          D: Debug + 'static,
          E: Debug + 'static,
          F: Debug + 'static,
          G: Debug + 'static,
          H: Debug + 'static,
          I: Debug + 'static,
          J: Debug + 'static,
          K: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.and_thenf(|c| {
                fd.and_thenf(|d| {
                    fe.and_thenf(|e| {
                        ff.and_thenf(|f| {
                            fg.and_thenf(|g| {
                                fh.and_thenf(|h| {
                                    fi.and_thenf(|i| {
                                        fj.and_thenf(|j| {
                                            fk.map(|k| (a, b, c, d, e, f, g, h, i, j, k))
                                        })
                                    })
                                })
                            })
                        })
                    })
                })
            })
        })
    })
}

pub fn join12<A, B, C, D, E, F, G, H, I, J, K, L, ERR>(
    fa: Future<A, ERR>,
    fb: Future<B, ERR>,
    fc: Future<C, ERR>,
    fd: Future<D, ERR>,
    fe: Future<E, ERR>,
    ff: Future<F, ERR>,
    fg: Future<G, ERR>,
    fh: Future<H, ERR>,
    fi: Future<I, ERR>,
    fj: Future<J, ERR>,
    fk: Future<K, ERR>,
    fl: Future<L, ERR>,
) -> Future<(A, B, C, D, E, F, G, H, I, J, K, L), ERR>
    where A: Debug + 'static,
          B: Debug + 'static,
          C: Debug + 'static,
          D: Debug + 'static,
          E: Debug + 'static,
          F: Debug + 'static,
          G: Debug + 'static,
          H: Debug + 'static,
          I: Debug + 'static,
          J: Debug + 'static,
          K: Debug + 'static,
          L: Debug + 'static,
          ERR: Debug + 'static
{
    fa.and_thenf(|a| {
        fb.and_thenf(|b| {
            fc.and_thenf(|c| {
                fd.and_thenf(|d| {
                    fe.and_thenf(|e| {
                        ff.and_thenf(|f| {
                            fg.and_thenf(|g| {
                                fh.and_thenf(|h| {
                                    fi.and_thenf(|i| {
                                        fj.and_thenf(|j| {
                                            fk.and_thenf(|k| {
                                                fl.map(|l| (a, b, c, d, e, f, g, h, i, j, k, l))
                                            })
                                        })
                                    })
                                })
                            })
                        })
                    })
                })
            })
        })
    })
}
