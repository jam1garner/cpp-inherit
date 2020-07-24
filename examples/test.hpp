struct non_virtual {
    int value;
    int x();
    non_virtual(int v);
};

struct base {
    int value;
    virtual int x();
    base(int v);
};

struct derived : public base {
    virtual int x();
    derived(int v);
};

int call_x_on(base* x);
