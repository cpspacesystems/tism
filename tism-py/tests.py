import tism
import unittest

class Tests(unittest.TestCase):
    def test_create(self):
        shm = tism.create("test_create_shm", bytes([128]))

        self.assertEqual(bytes([128]), shm.read())
        shm.write(bytes([65]))
        self.assertEqual(bytes([65]), shm.read())

        del shm

        
    def test_open(self):
        owner = tism.create("test_open_shm", bytes([0xBE, 0xEF]))
        borrower = tism.open("test_open_shm")

        self.assertEqual(bytes([0xBE, 0xEF]), borrower.read())
        owner.write(bytes([0xFA, 0xAA]))
        self.assertEqual(bytes([0xFA, 0xAA]), borrower.read())

        del borrower
        del owner


    def test_wait_and_open(self):
        owner = tism.create("test_wait_shm", bytes([0xBE, 0xEF]))
        borrower = tism.wait_and_open("test_wait_shm")

        self.assertEqual(bytes([0xBE, 0xEF]), borrower.read())
        owner.write(bytes([0xFA, 0xAA]))
        self.assertEqual(bytes([0xFA, 0xAA]), borrower.read())

        del borrower
        del owner


if __name__ == "__main__":
    unittest.main()
