// file_operations.c
#include <stdio.h>
#include <stdlib.h>

void read_file(const char *filename) {
    FILE *file = fopen(filename, "r");
    if (!file) {
        perror("Unable to open file");
        exit(1);
    }

    char ch;
    while ((ch = fgetc(file)) != EOF) {
        putchar(ch);
    }

    fclose(file);
}

int main() {
    const char *filename = "example.txt";
    read_file(filename);
    return 0;
}

