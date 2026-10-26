import com.memeasaur.untitled.*;
import java.util.Random;
public class ReferenceTodoAi {
    public static void main(String[] args) {
        System.out.println("# mode,min,max,seed,FNV1a64 over 600 source intervals (little endian i64)");
        for (int mode=0;mode<3;mode++) for (int[] bounds: new int[][]{{1,1},{6,13},{10,10},{21,25}}) for (long seed: new long[]{0,42,987654321}) {
            long[] now={1000000};
            var timing=new VapeAutoClickerTimingTodoAi(new Random(seed),new VapeSilentProfileTodoAi(seed,()->now[0]));
            long hash=0xcbf29ce484222325L;
            for(int i=0;i<600;i++) {
                if (i>0) now[0]+=i%100==0?35000:(i%37==0?1500:123+i%13);
                long value=timing.interval(bounds[0],bounds[1],mode);
                for(int j=0;j<8;j++) { hash^=(value>>>(j*8))&255; hash*=0x100000001b3L; }
            }
            System.out.printf("%d,%d,%d,%d,%016x%n",mode,bounds[0],bounds[1],seed,hash);
        }
    }
}
