# Examples

## Reactive Variables

```lua
func main(){
    x = 1;
    y ::= x + 1;

    println y;   # 2 #
    x = 10;
    println y;   # 11 #
}
```

## Struct with Reactive Fields

```lua
struct Counter {
    x = 0;
    step := 1;
    next ::= x + step;
}

func main(){
    c = struct Counter;
    println c.next; # 1 #
    c.x = 10;
    println c.next; # 11 #
}
```

## Factorial via Dependency Graph

```lua
func main(){
    fact = [6];

    fact[0] ::= 1;
    fact[1] ::= 1;

    x = 1;
    dx ::= x + 1;

    loop {
        if x >= fact - 1 {
            break;
        }

        i := x;
        fact[i + 1] ::= fact[i] * (i + 1);
        x = dx;
    }

    println fact[5];  # 120 #
}
```

## Arrays and Lazy Elements

```lua
func main(){
    arr = [5];
    x = 2;

    arr[0] ::= x * 10;
    println arr[0];  # 20 #

    x = 7;
    println arr[0];  # 70 #
}
```

## Bouncing String via Reactive Framebuffer

```lua
struct Screen {
    width;
    height;
    buf;
}

struct Text {
    str;
    len;

    x = 0;
    y = 0;
    vx = 1;
    vy = 1;

    dx ::= x + vx;
    dy ::= y + vy;
}

func make_text(str){
    text := struct Text;
    text.str := str;
    text.len ::= text.str;
    return text;
}

func make_screen(width, height) {
    screen := struct Screen;
    screen.width := width;
    screen.height := height;
    screen.buf := [screen.height];

    y = 0;
    dy ::= y + 1;

    loop {
        if y >= screen.height { break; }
        screen.buf[y] = [screen.width];
        y = dy;
    }
    return screen;
}

func framebuffer(screen, text) {
    y = 0;
    dy ::= y + 1;

    loop {
        if y >= screen.height { break; }

        x = 0;
        dx ::= x + 1;

        loop {
            if x >= screen.width { break; }

            yy := y;
            xx := x;

            screen.buf[yy][xx] ::=
                (yy == text.y &&
                 xx >= text.x &&
                 xx < text.x + text.len)
                    ? text.str[xx - text.x]
                    : (' ');

            x = dx;
        }

        y = dy;
    }
}

func render(screen) {
    print "\033[2J";
    print "\033[H";

    y = 0;
    dy ::= y + 1;

    loop {
        if y >= screen.height { break; }
        println screen.buf[y];
        y = dy;
    }
}

func delay(n) {
    d = 0;
    dd ::= d + 1;

    loop {
        if d >= n { break; }
        d = dd;
    }
}

func main(){
    text := make_text("HELLO REACTIVE");
    screen := make_screen(31,5);

    framebuffer(screen, text);

    loop {
        render(screen);
        delay(20000);

        text.x = text.dx;
        text.y = text.dy;

        if text.x < 0 {
            text.x = -text.x;
            text.vx = -text.vx;
        }

        if (text.x + text.len) > screen.width {
            text.x = (screen.width - text.len) - ((text.x + text.len) - screen.width);
            text.vx = -text.vx;
        }

        if text.y < 0 {
            text.y = -text.y;
            text.vy = -text.vy;
        }

        if text.y > (screen.height - 1) {
            text.y = (screen.height - 1) - (text.y - (screen.height - 1));
            text.vy = -text.vy;
        }
    }
}
```
