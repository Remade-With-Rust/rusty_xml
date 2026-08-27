#define LIBXML_STATIC
#include <stdio.h>
#include <libxml/chvalid.h>
int main(void) {
    unsigned i;
    FILE *f = fopen("c:/Users/talmo/coding/rusty_xml/corpora/xmlIsChar-bmp.bin", "wb");
    if (!f) return 1;
    for (i = 0; i <= 0xFFFFu; i++) {
        unsigned char b = (unsigned char)xmlIsCharQ(i);
        fwrite(&b, 1, 1, f);
    }
    fclose(f);
    return 0;
}
