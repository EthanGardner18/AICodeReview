// file_search.kt
import java.io.File

fun searchFiles(directory: String, keyword: String) {
    val dir = File(directory)
    if (dir.exists() && dir.isDirectory) {
        dir.walk().forEach {
            if (it.isFile && it.readText().contains(keyword)) {
                println("Found in: ${it.path}")
            }
        }
    } else {
        println("Directory not found")
    }
}

fun main() {
    val directory = "./test_directory"
    val keyword = "TODO"
    searchFiles(directory, keyword)
}

